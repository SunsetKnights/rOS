#![no_std]
#![no_main]

use user_lib::{get_task_info, get_time, yield_};

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    let current_timer = get_time();
    let wait_for = current_timer + 5000;
    while get_time() < wait_for {
        yield_();
    }
    for task_id in 0..=4 {
        let task_info = get_task_info(task_id);
        println!("{:#?}", task_info);
    }
    0
}
