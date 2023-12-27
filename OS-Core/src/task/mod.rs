use self::switch::__switch;
use self::task::TaskControlBlock;
use crate::config::{SYSCALL_ID, SYSCALL_QUANTITY};
use crate::loader::{get_num_app, load_app};
use crate::sbi_services::shutdown;
use crate::sync::UPSafeCell;
use crate::task::context::TaskContext;
use crate::task::info::TaskInfo;
use crate::task::task::TaskStatus;
use crate::timer::get_time;
use crate::trap::TrapContext;
use crate::println;
use alloc::vec::Vec;
use lazy_static::lazy_static;

pub mod context;
pub mod info;
pub mod switch;
pub mod task;

// constant
pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}
// variable
pub struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    tasks_info: Vec<TaskInfo>,
    current_task: usize,
    last_trap_time: usize,
}

impl TaskManager {
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        inner.tasks_info[0].status = TaskStatus::Running;
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let task0_context_ptr = &task0.task_context as *const TaskContext;
        let mut fake_task_ptr = TaskContext::zero_init();
        drop(inner);
        unsafe { __switch((&mut fake_task_ptr) as *mut TaskContext, task0_context_ptr) };
        panic!("unreachable in run_first_task!");
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let task = inner.current_task;
        inner.tasks[task].task_status = TaskStatus::Ready;
        inner.tasks_info[task].status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let task = inner.current_task;
        inner.tasks[task].task_status = TaskStatus::Exited;
        inner.tasks_info[task].status = TaskStatus::Exited;
    }

    fn called_system_call(&self, sys_call_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let task = inner.current_task;
        for i in 0..SYSCALL_QUANTITY {
            if SYSCALL_ID[i] == sys_call_id {
                inner.tasks_info[task].call[i].time += 1;
                break;
            }
        }
    }

    fn leave_kernel(&self) {
        self.inner.exclusive_access().last_trap_time = get_time();
    }

    fn entry_kernel(&self) {
        let curr_time = get_time();
        let mut inner = self.inner.exclusive_access();
        let curr_task_id = inner.current_task;
        inner.tasks_info[curr_task_id].time += curr_time - inner.last_trap_time;
    }

    fn get_task_info(&self, id: usize, ti: *mut TaskInfo) {
        let inner = self.inner.exclusive_access();
        unsafe {
            (*ti) = inner.tasks_info[id].clone();
        }
    }

    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        inner.tasks[current_task].get_user_token()
    }

    fn get_current_trap_context(&self) -> &mut TrapContext {
        let inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        inner.tasks[current_task].get_trap_context()
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        (current_task + 1..current_task + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Switch the next task status to running and switch to the next task.
    /// The status of the current task is determined by the place where this function is called.
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            // set next task status to running
            inner.tasks[next].task_status = TaskStatus::Running;
            // get two task context pointer
            let current_task_id = inner.current_task;
            let current_task_context_ptr =
                &mut inner.tasks[current_task_id].task_context as *mut TaskContext;
            let next_task_context_ptr = &inner.tasks[next].task_context as *const TaskContext;
            // set new curr task
            inner.current_task = next;
            drop(inner);
            // changed task, so this function can't finish, drop inner manually
            unsafe {
                __switch(current_task_context_ptr, next_task_context_ptr);
            }
        } else {
            println!("All applications completed!");
            shutdown();
        }
    }
}

lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        let mut tasks_info: Vec<TaskInfo> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(load_app(i), i));
            tasks_info.push(TaskInfo::init(i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    tasks_info,
                    current_task: 0,
                    last_trap_time: 0,
                })
            },
        }
    };
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.mark_current_exited();
    TASK_MANAGER.run_next_task();
}

pub fn suspended_current_and_run_next() {
    TASK_MANAGER.mark_current_suspended();
    TASK_MANAGER.run_next_task();
}

pub fn called_system_call(system_call_id: usize) {
    TASK_MANAGER.called_system_call(system_call_id);
}

pub fn record_leave_kernel_time() {
    TASK_MANAGER.leave_kernel();
}

pub fn update_user_task_run_time() {
    TASK_MANAGER.entry_kernel();
}

pub fn get_task_info(id: usize, ti: *mut TaskInfo) {
    TASK_MANAGER.get_task_info(id, ti);
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_user_trap_context() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_context()
}
