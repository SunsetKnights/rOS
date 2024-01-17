#![allow(unused)]

// here is eid and fid for sbi call:
// https://www.scs.stanford.edu/~zyedidia/docs/riscv/riscv-sbi.pdf
const EID_SET_TIMER: usize = 0;
const EID_CONSOLE_PUTCHAR: usize = 1;
const EID_CONSOLE_GETCHAR: usize = 2;
const FID_DEFAULT: usize = 0;

const EID_SRST_EXTENSION: usize = 0x53525354;
const FID_SHUTDOWN: usize = 0;

const EID_TIMER_EXTENSION: usize = 0x54494D45;
const FID_SET_TIMER: usize = 0;

use core::{arch::asm, result};

use crate::println;

#[inline(always)]
fn sbi_call(extension_id: usize, function_id: usize, args: [usize; 6]) -> isize {
    let result;
    unsafe {
        asm! {
            "ecall",
            inlateout("a0") args[0]=>result,
            in("a1") args[1],
            in("a2") args[2],
            in("a3") args[3],
            in("a4") args[4],
            in("a5") args[5],
            in("a6") function_id,
            in("a7") extension_id,
        }
    }
    result
}

/// Put a char on screen.
pub fn console_putchar(c: usize) {
    sbi_call(EID_CONSOLE_PUTCHAR, FID_DEFAULT, [c, 0, 0, 0, 0, 0]);
}

/// Get a char from console.
pub fn console_getchar() -> u8 {
    let result = sbi_call(EID_CONSOLE_GETCHAR, FID_DEFAULT, [0; 6]);
    assert_ne!(result, -1, "Read char error from sbi.");
    result as u8
}

/// Shutdown.
pub fn shutdown() -> ! {
    sbi_call(EID_SRST_EXTENSION, FID_SHUTDOWN, [0; 6]);
    panic!("shutdown command executed!");
}

/// Set value of mtimecmp register.
pub fn set_timer(timer: usize) {
    sbi_call(EID_TIMER_EXTENSION, FID_SET_TIMER, [timer, 0, 0, 0, 0, 0]);
}
