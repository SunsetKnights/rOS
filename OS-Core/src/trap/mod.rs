mod context;
use crate::{
    config::TRAMPOLINE,
    mm::address::VirtAddr,
    println,
    syscall::syscall,
    task::{
        check_current_signals_error, current_add_signal, exit_current_and_run_next,
        processor::{current_trap_context, current_trap_context_va, current_user_token},
        signal::SignalFlags,
        suspended_current_and_run_next,
    },
    timer::{check_timer, set_next_trigger},
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
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            check_timer();
            set_next_trigger();
            suspended_current_and_run_next();
        }
        Trap::Exception(Exception::UserEnvCall) => {
            let mut context = current_trap_context();
            context.sepc += 4;
            let ret =
                syscall(context.x[17], [context.x[10], context.x[11], context.x[12]]) as usize;
            context = current_trap_context();
            context.x[10] = ret;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            //println!("[kernel] PageFault in application, kernel killed it.");
            //// run next app
            //exit_current_and_run_next(-2);
            current_add_signal(SignalFlags::SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            //println!(
            //    "[kernel] IllegalInstruction in application, stval is {:#x}, kernel killed it.",
            //    stval
            //);
            //exit_current_and_run_next(-3);
            current_add_signal(SignalFlags::SIGILL);
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    };
    if let Some((err_code, err_info)) = check_current_signals_error() {
        println!("[kernel] {}", err_info);
        exit_current_and_run_next(err_code);
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
    let trap_context_ptr = current_trap_context_va();
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
