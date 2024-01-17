#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::{get_pid, yield_};

#[no_mangle]
pub fn main() -> i32 {
    println!("Hello, I am process {}.", get_pid());
    for i in 0..5 {
        yield_();
        println!("Back in process {}, iteration {}.", get_pid(), i);
    }
    println!("yield pass.");
    0
}
