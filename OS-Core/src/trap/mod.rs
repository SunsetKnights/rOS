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
    scause::{self, Exception, Interrupt, Trap},
    sie, sstatus, stval, stvec,
    utvec::TrapMode,
};

global_asm!(include_str!("trap.S"));

/// # Set CSR stvec
/// * When an exception occurs, the pc register will be set to the value in the stvec register,
/// that is, the value of the stvec register is the entry address into S mode.
pub fn init() {
    set_user_trap_entry();
}

/// Enable timer interrupt
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

/// Enable S mode interrupt (Kernel interrupt).
pub fn enable_smode_interrupt() {
    unsafe { sstatus::set_sie() };
}

/// Disable S mode interrupt (Kernel interrupt).
pub fn disable_smode_interrupt() {
    unsafe { sstatus::clear_sie() };
}

/// Set the user trap entry to trampoline page.
fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}

/// Set kernel trap entry (____save_kernel_trap_reg symbol in trampoline page).
fn set_kernel_trap_entry() {
    extern "C" {
        fn __savetrapsreg();
        fn __save_kernel_trap_reg();
    }
    let save_kernel_trap_va =
        TRAMPOLINE + (__save_kernel_trap_reg as usize - __savetrapsreg as usize);
    unsafe {
        stvec::write(save_kernel_trap_va, TrapMode::Direct);
        sscratch::write(trap_from_kernel as usize);
    }
}

/// User trap entry.
/// Handle trap from user.
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

/// Return to user space.
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

/// Kernel trap entry.
#[no_mangle]
pub fn trap_from_kernel() {
    todo!("handle external trap and timer trap")
}
