#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, get_pid, wait};

#[no_mangle]
pub fn main() -> i32 {
    println!("pid {}: parent start forking ...", get_pid());
    let pid = fork();
    if pid == 0 {
        // child process
        println!(
            "pid {}: forked child start execing hello_world app ... ",
            get_pid()
        );
        exec("hello_world\0");
        100
    } else {
        // parent process
        let mut exit_code: i32 = 0;
        println!("pid {}: ready waiting child ...", get_pid());
        assert_eq!(pid, wait(&mut exit_code));
        assert_eq!(exit_code, 0);
        println!(
            "pid {}: got child info:: pid {}, exit code: {}",
            get_pid(),
            pid,
            exit_code
        );
        0
    }
}
