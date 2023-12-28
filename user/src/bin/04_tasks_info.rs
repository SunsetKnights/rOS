#![no_std]
#![no_main]

use user_lib::get_task_info;

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    for task_id in 0..=4 {
        let task_info = get_task_info(task_id);
        println!("{:#?}", task_info);
    }
    0
}
