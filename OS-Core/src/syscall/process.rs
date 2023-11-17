// process manage mod

use crate::println;

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    // here need that run next app function
    todo!()
}
