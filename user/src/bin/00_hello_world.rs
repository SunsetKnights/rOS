#![no_std]
#![no_main]
#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("*******00 start*******");
    println!("hello, world!");
    println!("********00 end********");
    0
}