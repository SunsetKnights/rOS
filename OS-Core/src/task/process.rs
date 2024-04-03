use core::cell::RefMut;

use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};

use crate::{
    fs::{File, Stdin, Stdout},
    mm::{
        memory_set::{MemorySet, KERNEL_SPACE},
        page_table::PageTable,
    },
    sync::{condvar::Condvar, mutex::Mutex, semaphore::Semaphore, UPSafeCell},
    task::res::pid_alloc,
    trap::{trap_handler, TrapContext},
};

use super::{
    manager::{add_proc, add_ready_thread},
    res::{IdAlloctor, PidHandle, SequenceAllocator},
    signal::SignalFlags,
    thread::ThreadControlBlock,
};

pub struct ProcessControlBlock {
    pub pid: PidHandle,
    pub user_stack_base: usize,
    pub inner: UPSafeCell<ProcessControlBlockInner>,
}
impl ProcessControlBlock {
    /// Create a new process with main thread.
    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        // Alloc pid and memory set for process.
        let (memory_set, user_stack_base, entry_point) = MemorySet::new_app_from_elf(elf_data);
        let pid = pid_alloc();
        let pcb = Self {
            pid,
            user_stack_base,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: vec![
                        //stdin
                        Some(Arc::new(Stdin)),
                        //stdout
                        Some(Arc::new(Stdout)),
                        //stderr
                        Some(Arc::new(Stdout)),
                    ],
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    signals: SignalFlags::empty(),
                    thread_res_allocator: SequenceAllocator::new(),
                    threads: Vec::new(),
                })
            },
        };
        let process = Arc::new(pcb);
        // Create the main thread.
        let main_thread = Arc::new(ThreadControlBlock::new(
            process.clone(),
            user_stack_base,
            true,
        ));
        let thread_inner = main_thread.inner_exclusive_access();
        let trap_context = thread_inner.trap_context();
        let user_stack_bottom = thread_inner.res.as_ref().unwrap().user_stack_bottom();
        let kernel_stack_bottom = main_thread.kernel_stack.get_bottom();
        drop(thread_inner);
        *trap_context = TrapContext::init_app_context(
            entry_point,
            user_stack_bottom,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_bottom,
            trap_handler as usize,
        );
        // Add thread to process.
        process
            .inner_exclusive_access()
            .threads
            .push(Some(main_thread.clone()));
        // Add process and thread to manager
        add_proc(process.clone());
        add_ready_thread(main_thread);
        process
    }
    /// Replace the program of the current process with the specified program.
    /// Only support single thread process.
    pub fn exec(self: &Arc<Self>, elf_data: &[u8], args: Vec<String>) {
        assert!(self.inner_exclusive_access().thread_count() == 1);
        // New memory set
        let (memory_set, user_stack_base, entry_point) = MemorySet::new_app_from_elf(elf_data);
        let token = memory_set.token();
        self.inner_exclusive_access().memory_set = memory_set;
        // Alloc new thread user resource in new memory set.
        let main_thread = self.inner_exclusive_access().get_thread(0);
        let mut thread_inner = main_thread.inner_exclusive_access();
        // The old resource (user stack and trap page) was already recycled when the memory set droped.
        thread_inner.realloc_res(user_stack_base);
        let mut user_sp = thread_inner.res.as_ref().unwrap().user_stack_bottom();

        // Put arguments to user stack
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>(); // args ptr
        let arg_ptr_base = user_sp;
        // The position of the arg ptr in the user stack.
        let mut arg_ptr_pos_vec: Vec<&mut usize> = (0..=args.len())
            .map(|arg_num| {
                PageTable::from_token(token).translated_refmut(
                    (arg_ptr_base + arg_num * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *arg_ptr_pos_vec[args.len()] = 0;
        // Put all arg in user stack.
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *arg_ptr_pos_vec[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *PageTable::from_token(token).translated_refmut(p as *mut u8) = *c;
                p += 1;
            }
            *PageTable::from_token(token).translated_refmut(p as *mut u8) = '\0' as u8;
        }
        // Align user_sp to sizeof(usize)
        user_sp -= user_sp % core::mem::size_of::<usize>();

        let mut trap_context = TrapContext::init_app_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            main_thread.kernel_stack.get_bottom(),
            trap_handler as usize,
        );
        trap_context.x[10] = args.len();
        trap_context.x[11] = arg_ptr_base;
        *thread_inner.trap_context() = trap_context;
    }
    /// Fork a new process.
    /// Only support single thread process.
    pub fn fork(self: &Arc<ProcessControlBlock>) -> Arc<ProcessControlBlock> {
        assert!(self.inner_exclusive_access().thread_count() == 1);
        // Create child process.
        let mut parent_inner = self.inner_exclusive_access();
        let memory_set = parent_inner.memory_set.clone();
        let fd_table = parent_inner.fd_table.clone();
        let user_stack_base = self.user_stack_base;
        let pid_handle = pid_alloc();
        let child_proc = Arc::new(Self {
            pid: pid_handle,
            user_stack_base,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table,
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    signals: SignalFlags::empty(),
                    thread_res_allocator: SequenceAllocator::new(), // Single thread, don't need clone thread allocator
                    threads: Vec::new(),
                })
            },
        });
        // Create main thread for child process.
        let parent_main_thread = parent_inner.get_thread(0);
        let parent_thread_inner = parent_main_thread.inner_exclusive_access();
        let child_main_thread = Arc::new(ThreadControlBlock::new(
            child_proc.clone(),
            user_stack_base,
            false,
        ));
        let kernel_stack_bottom = child_main_thread.kernel_stack.get_bottom();
        child_main_thread
            .inner_exclusive_access()
            .trap_context()
            .kernel_sp = kernel_stack_bottom;
        drop(parent_thread_inner);
        // Push child main thread to child process.
        child_proc
            .inner_exclusive_access()
            .threads
            .push(Some(child_main_thread.clone()));
        // Add child process
        parent_inner.children.push(child_proc.clone());
        drop(parent_inner);
        add_ready_thread(child_main_thread);
        add_proc(child_proc.clone());
        child_proc
    }
    /// Create a child process and run the elf program.
    /// Only support single thread process.
    pub fn spawn(self: &Arc<ProcessControlBlock>, elf_data: &[u8], args: Vec<String>) -> usize {
        assert!(self.inner_exclusive_access().thread_count() == 1);
        let fd_table = self.inner_exclusive_access().fd_table.clone();
        // Alloc pid and memory set for process.
        let (memory_set, user_stack_base, entry_point) = MemorySet::new_app_from_elf(elf_data);
        let pid = pid_alloc();
        let result = pid.0;
        let token = memory_set.token();
        let child_proc = Arc::new(Self {
            pid,
            user_stack_base,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table,
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    signals: SignalFlags::empty(),
                    thread_res_allocator: SequenceAllocator::new(),
                    threads: Vec::new(),
                })
            },
        });
        // Create the main thread.
        let main_thread = Arc::new(ThreadControlBlock::new(
            child_proc.clone(),
            user_stack_base,
            true,
        ));
        let thread_inner = main_thread.inner_exclusive_access();
        let mut user_sp = thread_inner.res.as_ref().unwrap().user_stack_bottom();

        // Put arguments to user stack
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>(); // args ptr
        let arg_ptr_base = user_sp;
        // The position of the arg ptr in the user stack.
        let mut arg_ptr_pos_vec: Vec<&mut usize> = (0..=args.len())
            .map(|arg_num| {
                PageTable::from_token(token).translated_refmut(
                    (arg_ptr_base + arg_num * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *arg_ptr_pos_vec[args.len()] = 0;
        // Put all arg in user stack.
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *arg_ptr_pos_vec[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *PageTable::from_token(token).translated_refmut(p as *mut u8) = *c;
                p += 1;
            }
            *PageTable::from_token(token).translated_refmut(p as *mut u8) = '\0' as u8;
        }
        // Align user_sp to sizeof(usize)
        user_sp -= user_sp % core::mem::size_of::<usize>();

        let kernel_stack_bottom = main_thread.kernel_stack.get_bottom();
        let trap_context = thread_inner.trap_context();
        drop(thread_inner);
        *trap_context = TrapContext::init_app_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_bottom,
            trap_handler as usize,
        );
        // Add thread to process.
        child_proc
            .inner_exclusive_access()
            .threads
            .push(Some(main_thread.clone()));
        // Add process and thread to manager
        add_proc(child_proc.clone());
        add_ready_thread(main_thread);
        result
    }
    pub fn token(&self) -> usize {
        self.inner_exclusive_access().memory_set.token()
    }
    pub fn pid(&self) -> usize {
        self.pid.0
    }
    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }
}

pub struct ProcessControlBlockInner {
    pub is_zombie: bool,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    pub condvar_list: Vec<Option<Arc<Condvar>>>,
    pub signals: SignalFlags,
    pub thread_res_allocator: SequenceAllocator,
    pub threads: Vec<Option<Arc<ThreadControlBlock>>>,
}
impl ProcessControlBlockInner {
    pub fn user_token(&self) -> usize {
        self.memory_set.token()
    }
    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
    pub fn open_file(&mut self, file: Arc<dyn File + Send + Sync>) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            self.fd_table[fd] = Some(file);
            fd
        } else {
            self.fd_table.push(Some(file));
            self.fd_table.len() - 1
        }
    }
    pub fn add_mutex(&mut self, mutex: Arc<dyn Mutex>) -> usize {
        if let Some(mutex_id) =
            (0..self.mutex_list.len()).find(|&mutex_id| self.mutex_list[mutex_id].is_none())
        {
            self.mutex_list[mutex_id] = Some(mutex);
            mutex_id
        } else {
            self.mutex_list.push(Some(mutex));
            self.mutex_list.len() - 1
        }
    }
    pub fn add_semaphore(&mut self, semaphore: Arc<Semaphore>) -> usize {
        if let Some(semaphore_id) = (0..self.semaphore_list.len())
            .find(|&semaphore_id| self.semaphore_list[semaphore_id].is_none())
        {
            self.semaphore_list[semaphore_id] = Some(semaphore);
            semaphore_id
        } else {
            self.semaphore_list.push(Some(semaphore));
            self.semaphore_list.len() - 1
        }
    }
    pub fn add_condvar(&mut self, condvar: Arc<Condvar>) -> usize {
        if let Some(condvar_id) =
            (0..self.condvar_list.len()).find(|&condvar_id| self.condvar_list[condvar_id].is_none())
        {
            self.condvar_list[condvar_id] = Some(condvar);
            condvar_id
        } else {
            self.condvar_list.push(Some(condvar));
            self.condvar_list.len() - 1
        }
    }
    pub fn alloc_tid(&mut self) -> usize {
        self.thread_res_allocator.alloc()
    }
    pub fn dealloc_tid(&mut self, tid: usize) {
        self.thread_res_allocator.dealloc(tid);
    }
    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }
    pub fn get_thread(&self, tid: usize) -> Arc<ThreadControlBlock> {
        self.threads[tid].as_ref().unwrap().clone()
    }
    pub fn get_mutex(&self, mutex_id: usize) -> Arc<dyn Mutex> {
        self.mutex_list[mutex_id].as_ref().unwrap().clone()
    }
    pub fn get_semaphore(&self, semaphore_id: usize) -> Arc<Semaphore> {
        self.semaphore_list[semaphore_id].as_ref().unwrap().clone()
    }
    pub fn get_condvar(&self, condvar_id: usize) -> Arc<Condvar> {
        self.condvar_list[condvar_id].as_ref().unwrap().clone()
    }
}
