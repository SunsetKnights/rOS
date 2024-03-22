#![no_std]
#![no_main]

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use user_lib::{close, console::get_char, dup, exec, fork, open, wait_pid, OpenFlags};

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

                // Split commands and parameters.
                let args: Vec<&str> = line.as_str().split(' ').collect();
                let mut io_redirect_symbol_idx = None;
                let mut args_with_end: Vec<String> = args
                    .iter()
                    .enumerate()
                    .map(|(idx, &arg)| {
                        // Processing stream operators
                        if arg == ">" || arg == "<" {
                            io_redirect_symbol_idx = Some(idx);
                        }
                        let mut arg_string = arg.to_string();
                        arg_string.push('\0');
                        arg_string
                    })
                    .collect();

                // Get the io redirected file
                let mut input = String::new();
                let mut output = String::new();
                if let Some(idx) = io_redirect_symbol_idx {
                    let mut drain = args_with_end.drain(idx..=idx + 1);
                    let symbol = drain.next().unwrap();
                    let redirect_file = drain.next().unwrap();
                    if symbol == "<" {
                        input = redirect_file;
                    } else {
                        output = redirect_file;
                    }
                }

                let mut args_addr: Vec<*const u8> =
                    args_with_end.iter().map(|s| s.as_ptr()).collect();
                args_addr.push(0 as *const u8);
                let pid = fork();
                // child process
                if pid == 0 {
                    if !input.is_empty() {
                        let input_fd = open(&input, OpenFlags::READ_ONLY);
                        if input_fd == -1 {
                            println!("Error when opening file {}.", input);
                            return -4;
                        }
                        // close stdin
                        close(0);
                        // copy input file to fd 0
                        assert_eq!(dup(input_fd as usize), 0, "Error when input redirect");
                        close(input_fd as usize);
                    }

                    if !output.is_empty() {
                        let output_fd = open(&output, OpenFlags::WRITE_ONLY | OpenFlags::CREATE);
                        if output_fd == -1 {
                            println!("Error when opening file {}.", output);
                            return -4;
                        }
                        // close stdout
                        close(1);
                        // copy output file to fd 1
                        assert_eq!(dup(output_fd as usize), 1, "Error when output redirect");
                        close(output_fd as usize);
                    }

                    if exec(args_with_end[0].as_str(), args_addr.as_slice()) == -1 {
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
                print!(">> ");
                line.clear();
            }
            BS | DL => {
                if line.pop().is_some() {
                    print!("{}", BS as char);
                    print!(" ");
                    print!("{}", BS as char);
                }
            }
            32..=126 => {
                print!("{}", c as char);
                line.push(c as char);
            }
            _ => {
                print!("{}", c as char);
            }
        }
    }
}
