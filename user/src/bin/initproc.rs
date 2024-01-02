#![no_std]
#![no_main]

use user_lib::{exec, fork, wait, yield_};

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    // The current process is a child process
    if fork() == 0 {
        exec("user_shell\0");
    }
    // The current process is the parent process and waits in a loop for all child processes to exit.
    else {
        loop {
            let mut exit_code = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_();
                continue;
            }
            println!(
                "[initproc] Released a zombie process, pid={}, exit_code={}.",
                pid, exit_code
            );
        }
    }
    0
}
