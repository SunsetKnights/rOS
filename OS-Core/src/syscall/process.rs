// process manage mod

use crate::{batch::run_next_app, println};

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    // It may be necessary to delete the last UserContext saved in the kernel stack
    run_next_app();
}
