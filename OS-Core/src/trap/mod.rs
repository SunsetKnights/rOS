mod context;

use crate::{println, syscall::syscall};
pub use context::TrapContext;
use core::arch::global_asm;
use riscv::register::{
    scause::{self, Exception, Trap},
    stval, stvec,
    utvec::TrapMode,
};

global_asm!(include_str!("trap.S"));

/// # Set CSR stvec
/// * When an exception occurs, the pc register will be set to the value in the stvec register,
/// that is, the value of the stvec register is the entry address into S mode.
pub fn init() {
    extern "C" {
        fn __savetrapsreg();
    }
    unsafe {
        stvec::write(__savetrapsreg as usize, TrapMode::Direct);
    }
}

///  Process the trap.
///  If it is a system call, call the syscall function.
///  If it is an exception, print the exception information and then execute the next program.
///
/// # Parameter
/// * 'context' - User stack context
#[no_mangle]
pub fn trap_handler(context: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            context.sepc += 4;
            context.x[10] =
                syscall(context.x[17], [context.x[10], context.x[11], context.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, kernel killed it.");
            // run next app
            todo!();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            todo!()
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    context
}
