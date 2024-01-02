#![no_std]
#![no_main]

use alloc::string::String;
use user_lib::{console::get_char, exec, fork, wait_pid};

extern crate alloc;
#[macro_use]
extern crate user_lib;

const BS: u8 = 0x08u8; // backspace
const LF: u8 = 0x0au8; // \n
const CR: u8 = 0x0du8; // \r
const DL: u8 = 0x7fu8; // delete

#[no_mangle]
fn main() -> i32 {
    println!("Welcom to rust shell");
    print!(">> ");
    let mut line = String::new();
    loop {
        let c = get_char();
        match c {
            LF | CR => {
                println!("");
                if line.is_empty() {
                    print!(">> ");
                    continue;
                }
                line.push('\0');
                let pid = fork();
                // child process
                if pid == 0 {
                    if exec(line.as_str()) == -1 {
                        println!("Error when executing");
                        return -4;
                    }
                    unreachable!();
                }
                // parent process
                let mut exit_code = 0;
                let exit_pid = wait_pid(pid as usize, &mut exit_code);
                assert_eq!(pid, exit_pid);
                println!("Shell: Process {} exit with code {}.", pid, exit_code);
            }
            BS | DL => {
                print!("{}", BS as char);
                print!(" ");
                print!("{}", BS as char);
                line.pop();
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
