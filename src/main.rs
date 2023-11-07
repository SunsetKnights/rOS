#![no_std] //表示不用std，因为std需要system call，而是用core库
#![no_main] //表示没有main函数，因为没有std库，所以也不存在main函数之前的初始化过程
mod lang_runtimes; //完成核心（core）库里面的一些功能，例如panic宏

use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));
