use core::arch::asm;

use crate::{println, sbi_services::shutdown, sync::UPSafeCell, trap::TrapContext};
use lazy_static::lazy_static;

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};
static USER_STACK: UserStack = UserStack {
    data: [0; KERNEL_STACK_SIZE],
};

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, context: TrapContext) -> &'static mut TrapContext {
        let context_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *context_ptr = context;
        }
        unsafe { context_ptr.as_mut().unwrap() }
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

struct AppManager {
    num_app: usize,
    current_app: usize,
    // why MAX_APP_NUM plus 1??????
    app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManager {
    pub fn print_app_info(&self) {
        println!("[kernel] app quantity: {}", self.num_app);
        for i in 0..self.num_app {
            println!(
                "[kernel] app_{} memory:[{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    unsafe fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            println!("All applications completed!");
            shutdown();
        }
        println!("[kernel] Loading app_{}", app_id);
        // Clear application memory area
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
        // copy application memory slice to application start address(0x80400000)
        let app_src = core::slice::from_raw_parts(
            self.app_start[app_id] as *mut u8,
            self.app_start[app_id + 1] - self.app_start[app_id],
        );
        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);
        asm!("fence.i");
    }

    fn get_current_app(&self) -> usize {
        self.current_app
    }

    fn move_to_next_app(&mut self) {
        self.current_app += 1
    }
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            extern "C" {
                fn _num_app();
            }
            // this actually uses the symbols in the link_app.S file
            let num_app_ptr = _num_app as usize as *const usize;
            // .quad n; in link_app.S.
            // num_app is n
            let num_app = num_app_ptr.read_volatile();
            let mut app_start:[usize;MAX_APP_NUM+1] = [0;MAX_APP_NUM+1];
            let app_start_raw:&[usize] = core::slice::from_raw_parts(num_app_ptr.add(1), num_app+1);
            app_start[..=num_app].copy_from_slice(app_start_raw);
            AppManager { num_app, current_app: 0, app_start }
        })
    };
}

pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

/// Just print applications info currently
pub fn init() {
    print_app_info();
}

/// Clear user application memory aera, copy the user application data into user application memory area.
/// Initialize UserContext, put it in kernel stack and set spec register so that the sret instruction can return correctly.
pub fn run_next_app() -> ! {
    let mut app_manager = APP_MANAGER.exclusive_access();
    let curr_app = app_manager.get_current_app();
    unsafe {
        app_manager.load_app(curr_app);
    }
    app_manager.move_to_next_app();
    drop(app_manager);
    extern "C" {
        fn __restoretrapreg(context_addr: usize);
    }
    // init a new application context, put it in kernel stack
    // then, Use the __restoretrapreg to put the initial ApplicationContext in the kernel stack into the user stack
    unsafe {
        __restoretrapreg(KERNEL_STACK.push_context(TrapContext::init_app_context(
            APP_BASE_ADDRESS,
            USER_STACK.get_sp(),
        )) as *const _ as usize);
    }
    panic!("Unreachable in batch::run_current_app!");
}
