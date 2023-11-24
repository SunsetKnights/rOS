// easy to call switch

use super::context::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.S"));

extern "C" {
    /// Switch task, each task has different kernel stack and user stack.
    /// When the trap occurs, the sp of the user stack has been saved to the kernel stack.
    pub fn __switch(
        current_task_context_ptr: *mut TaskContext,
        next_task_context_ptr: *const TaskContext,
    );
}
