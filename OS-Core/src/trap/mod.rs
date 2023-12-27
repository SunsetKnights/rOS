mod context;

use crate::{
    config::{TRAMPOLINE, TRAP_CONTEXT},
    mm::address::VirtAddr,
    println,
    syscall::syscall,
    task::{
        called_system_call, current_user_token, current_user_trap_context,
        exit_current_and_run_next, suspended_current_and_run_next,
    },
    timer::set_next_trigger,
};
pub use context::TrapContext;
use core::arch::{asm, global_asm};
use riscv::register::{
    scause::Interrupt,
    scause::{self, Exception, Trap},
    sie, stval, stvec,
    utvec::TrapMode,
};

global_asm!(include_str!("trap.S"));

/// # Set CSR stvec
/// * When an exception occurs, the pc register will be set to the value in the stvec register,
/// that is, the value of the stvec register is the entry address into S mode.
pub fn init() {
    set_user_trap_entry();
}

/// enable timer interrupt
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

///  Process the trap.
///  If it is a system call, call the syscall function.
///  If it is an exception, print the exception information and then execute the next program.
///
/// # Parameter
/// * 'context' - User stack context
#[no_mangle]
pub fn trap_handler() {
    set_kernel_trap_entry();
    let context = current_user_trap_context();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspended_current_and_run_next();
        }
        Trap::Exception(Exception::UserEnvCall) => {
            context.sepc += 4;
            // Count the number of system calls
            called_system_call(context.x[17]);
            context.x[10] =
                syscall(context.x[17], [context.x[10], context.x[11], context.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, kernel killed it.");
            // run next app
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            exit_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    trap_return();
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}
#[no_mangle]
pub fn trap_from_kernel() {
    panic!("a trap from kernel!");
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}
#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_context_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();
    extern "C" {
        fn __restoretrapreg();
    }
    let restore_va = TRAMPOLINE + VirtAddr::from(__restoretrapreg as usize).page_offset();
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_context_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}
