#![no_std]
#![no_main]
#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("*******02 start*******");
    for i in 0..10 {
        if i & 1 == 1 {
            println!("{} is a odd number.", i);
        } else {
            println!("{} is a even number.", i);
        }
    }
    println!("Program that cycle switch S mode and U mode was finished.");
    println!("********02 end********");
    0
}
