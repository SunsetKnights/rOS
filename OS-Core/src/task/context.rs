#[derive(Clone, Copy)]
#[repr(C)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl TaskContext {
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    pub fn goto_restoretrapreg_init(kernel_stack_ptr: usize) -> Self {
        extern "C" {
            fn __restoretrapreg();
        }
        Self {
            ra: __restoretrapreg as usize,
            sp: kernel_stack_ptr,
            s: [0; 12],
        }
    }
}
