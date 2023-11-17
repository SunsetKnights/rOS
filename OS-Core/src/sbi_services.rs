#![allow(unused)]

// sbi的一些服务的代号，即service_type
const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_FENCE_VMA: usize = 6;
const SBI_REMOTE_FENCE_VMA_ASID: usize = 7;

const SRST_EXTENSION: usize = 0x53525354;
const SBI_SHUTDOWN: usize = 0;

use core::arch::asm;

#[inline(always)]
// 一个调用sbi服务的函数
// service_type表示请求服务的i类型，arg0-arg2表示三个参数，返回值会被放入arg0中
fn sbi_call(service_type: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut result;
    unsafe {
        asm! {
            "ecall",
            inlateout("x10") arg0=>result,
            in("x11") arg1,
            in("x12") arg2,
            in("x16") fid,
            in("x17") service_type,
        }
    }
    result
}

// 在屏幕上输出一个字符
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, 0, c, 0, 0);
}

// 关机
pub fn shutdown() -> ! {
    sbi_call(SRST_EXTENSION, SBI_SHUTDOWN, 0, 0, 0);
    panic!("shutdown command executed!");
}
