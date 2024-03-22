use super::{
    action::SignalActions,
    context::TaskContext,
    manager::add_task,
    pid::{pid_alloc, KernelStack, PidHandle},
    signal::SignalFlags,
};
use crate::{
    config::TRAP_CONTEXT,
    fs::{File, Stdin, Stdout},
    mm::{
        address::{PhysPageNum, VirtAddr},
        memory_set::{MemorySet, KERNEL_SPACE},
        page_table::PageTable,
    },
    sync::UPSafeCell,
    trap::{trap_handler, TrapContext},
};
use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use core::cell::RefMut;

#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Zombie,
    Exited,
}

pub struct ProcessControlBlock {
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    inner: UPSafeCell<ProcessControlBlockInner>,
}

impl ProcessControlBlock {
    /// Create a new process and run the elf program
    /// # Parameter
    /// * 'elf_data' - Elf program data.
    /// # Return
    /// * A new process control block
    pub fn new(elf_data: &[u8]) -> Self {
        let (memory_set, user_sp, entry_point) = MemorySet::new_app_from_elf(elf_data);
        let trap_context_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        let pid = pid_alloc();
        let kernel_stack = KernelStack::new(&pid);
        let kernel_stack_bottom = kernel_stack.get_bottom();
        let pcb = Self {
            pid,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    trap_context_ppn,
                    base_size: user_sp,
                    task_context: TaskContext::goto_trap_return(kernel_stack_bottom),
                    memory_set,
                    task_status,
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
                    signal_mask: SignalFlags::empty(),
                    signal_actions: SignalActions::default(),
                })
            },
        };
        let trap_context = pcb.inner_exclusive_access().get_trap_context();
        *trap_context = TrapContext::init_app_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_bottom,
            trap_handler as usize,
        );
        pcb
    }
    /// Get pid of current process control block.
    pub fn get_pid(&self) -> usize {
        self.pid.0
    }
    /// Gets a variable borrow for a variable section in a process control block
    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }
    /// Replace the program of the current process with the specified program.
    pub fn exec(&self, elf_data: &[u8], args: Vec<String>) {
        let (memory_set, mut user_sp, entry_point) = MemorySet::new_app_from_elf(elf_data);
        let trap_context_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // args ptr
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let arg_ptr_base = user_sp;
        // The position of the arg ptr in the user stack.
        let mut arg_ptr_pos_vec: Vec<&mut usize> = (0..=args.len())
            .map(|arg_num| {
                PageTable::from_token(memory_set.token()).translated_refmut(
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
                *PageTable::from_token(memory_set.token()).translated_refmut(p as *mut u8) = *c;
                p += 1;
            }
            *PageTable::from_token(memory_set.token()).translated_refmut(p as *mut u8) = '\0' as u8;
        }
        // Align user_sp to sizeof(usize)
        user_sp -= user_sp % core::mem::size_of::<usize>();

        let mut inner = self.inner_exclusive_access();
        inner.memory_set = memory_set;
        inner.trap_context_ppn = trap_context_ppn;
        let mut trap_context = TrapContext::init_app_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_bottom(),
            trap_handler as usize,
        );
        trap_context.x[10] = args.len();
        trap_context.x[11] = arg_ptr_base;
        *inner.get_trap_context() = trap_context;
    }
    /// Fork a new process.
    pub fn fork(self: &Arc<ProcessControlBlock>) -> Arc<ProcessControlBlock> {
        let mut parent_inner = self.inner_exclusive_access();
        let memory_set = parent_inner.memory_set.clone();
        let fd_table = parent_inner.fd_table.clone();
        let trap_context_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_bottom = kernel_stack.get_bottom();
        let child = Arc::new(Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    trap_context_ppn,
                    base_size: parent_inner.base_size,
                    task_context: TaskContext::goto_trap_return(kernel_stack_bottom),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table,
                    signal_mask: SignalFlags::empty(),
                    signal_actions: SignalActions::default(),
                })
            },
        });
        child.inner_exclusive_access().get_trap_context().kernel_sp = kernel_stack_bottom;
        parent_inner.children.push(child.clone());
        child
    }
    /// Create a child process and run the elf program
    pub fn spawn(self: &Arc<ProcessControlBlock>, elf_data: &[u8], args: Vec<String>) -> usize {
        // Copy parent process fd table
        let mut parent_inner = self.inner_exclusive_access();
        let fd_table = parent_inner.fd_table.clone();
        // Create a new memory set
        let (memory_set, mut user_sp, entry_point) = MemorySet::new_app_from_elf(elf_data);
        // Get trap context address
        let trap_context_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // args ptr
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let arg_ptr_base = user_sp;
        // The position of the arg ptr in the user stack.
        let mut arg_ptr_pos_vec: Vec<&mut usize> = (0..=args.len())
            .map(|arg_num| {
                PageTable::from_token(memory_set.token()).translated_refmut(
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
                *PageTable::from_token(memory_set.token()).translated_refmut(p as *mut u8) = *c;
                p += 1;
            }
            *PageTable::from_token(memory_set.token()).translated_refmut(p as *mut u8) = '\0' as u8;
        }
        // Align user_sp to sizeof(usize)
        user_sp -= user_sp % core::mem::size_of::<usize>();

        // Set trap context
        let mut trap_context = TrapContext::init_app_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_bottom(),
            trap_handler as usize,
        );
        trap_context.x[10] = args.len();
        trap_context.x[11] = arg_ptr_base;

        *trap_context_ppn.get_mut() = trap_context;

        let pid_handle = pid_alloc();
        let child_pid = pid_handle.0;
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_bottom = kernel_stack.get_bottom();
        let child = Arc::new(Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    trap_context_ppn,
                    base_size: parent_inner.base_size,
                    task_context: TaskContext::goto_trap_return(kernel_stack_bottom),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table,
                    signal_mask: SignalFlags::empty(),
                    signal_actions: SignalActions::default(),
                })
            },
        });

        parent_inner.children.push(child.clone());
        add_task(child);
        child_pid
    }
}

pub struct ProcessControlBlockInner {
    pub trap_context_ppn: PhysPageNum,
    pub base_size: usize,
    pub task_context: TaskContext,
    pub task_status: TaskStatus,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    pub signal_mask: SignalFlags,
    pub signal_actions: SignalActions,
}

impl ProcessControlBlockInner {
    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        self.trap_context_ppn.get_mut()
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    pub fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn is_zombie(&self) -> bool {
        self.task_status == TaskStatus::Zombie
    }
    pub fn get_task_context(&mut self) -> *mut TaskContext {
        &mut self.task_context as *mut TaskContext
    }
    pub fn set_task_status(&mut self, status: TaskStatus) {
        self.task_status = status;
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
}
