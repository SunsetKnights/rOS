use crate::{
    config::{kernel_stack_position, TRAP_CONTEXT},
    mm::{
        address::{PhysPageNum, VirtAddr},
        memory_set::{MapPermission, MemorySet, KERNEL_SPACE},
    },
    trap::{trap_handler, TrapContext},
};

use super::context::TaskContext;

#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

pub struct TaskControlBlock {
    pub task_context: TaskContext,
    pub task_status: TaskStatus,
    pub memory_set: MemorySet,
    pub trap_context_ppn: PhysPageNum,
    pub base_size: usize,
}

impl TaskControlBlock {
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        let (memory_set, user_sp, entry_point) = MemorySet::new_app_from_elf(elf_data);
        let trap_context_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_control_block = Self {
            task_context: TaskContext::goto_trap_return(kernel_stack_top),
            task_status,
            memory_set,
            trap_context_ppn,
            base_size: user_sp,
        };
        let trap_context = task_control_block.get_trap_context();
        *trap_context = TrapContext::init_app_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }

    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        self.trap_context_ppn.get_mut()
    }
}
