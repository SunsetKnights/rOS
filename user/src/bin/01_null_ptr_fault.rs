#![no_std]
#![no_main]
#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("*******01 start*******");
    println!("This program try to write something on the zero address.");
    println!("Kernel should kill this program.");
    unsafe{
        core::ptr::null_mut::<u8>().write_volatile(0);
    }
    0
}