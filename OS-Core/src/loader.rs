use core::arch::asm;

use crate::{config::*, trap::TrapContext};

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, context: TrapContext) -> usize {
        let context_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *context_ptr = context;
        }
        context_ptr as usize
    }
}

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];
static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// Load all apps to memory from .data section in kernel
pub fn load_apps() {
    //_num_app:
    //  .quad 5
    //  .quad app_0_start
    //  .quad app_1_start
    //  .quad app_2_start
    //  .quad app_3_start
    //  .quad app_4_start
    //  .quad app_4_end
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    unsafe {
        asm!("fence.i");
    }
    for i in 0..num_app {
        let dst_start = APP_BASE_ADDRESS + i * APP_SIZE_LIMIT;
        (dst_start..dst_start + APP_SIZE_LIMIT)
            .for_each(|x| unsafe { (x as *mut u8).write_volatile(0) });
        let app_size = app_start[i + 1] - app_start[i];
        let src = unsafe { core::slice::from_raw_parts(app_start[i] as *const u8, app_size) };
        let dst = unsafe { core::slice::from_raw_parts_mut(dst_start as *mut u8, app_size) };
        dst.copy_from_slice(src);
    }
}

pub fn init_app(app_id: usize) -> usize {
    KERNEL_STACK[app_id].push_context(TrapContext::init_app_context(
        APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT,
        USER_STACK[app_id].get_sp(),
    ))
}
