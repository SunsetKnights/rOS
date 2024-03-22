#![no_std] // use core
#![feature(linkage)] // use customed link script?
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod console;
mod lang_runtimes;
mod sys_call;

extern crate alloc;
#[macro_use]
extern crate bitflags;

use crate::sys_call::*;
use alloc::vec::Vec;
use buddy_system_allocator::LockedHeap;

const USER_HEAP_SIZE: usize = 0x4000;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::new();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[no_mangle]
// generation symbol use function name, no change
// set section name, in linker script, this section is top of all section. also, this symbol(function name) is the entry of program
#[link_section = ".text.entry"]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }

    let mut args: Vec<&'static str> = Vec::new();
    for i in 0..argc {
        let arg_i_start = unsafe { (argv as *const usize).add(i).read_volatile() } as *const u8;
        let mut arg_i_len = 0;
        while unsafe { arg_i_start.add(arg_i_len).read_volatile() } != '\0' as u8 {
            arg_i_len += 1;
        }
        args.push(
            core::str::from_utf8(unsafe { core::slice::from_raw_parts(arg_i_start, arg_i_len) })
                .unwrap(),
        );
    }

    exit(main(argc, args.as_slice())); //return exit code to os
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
fn main(_argc: usize, _argv: &[&str]) -> i32 {
    panic!("no main find.") //if not find main function in user program, use this main function
}

bitflags! {
    pub struct OpenFlags:u32{
        const READ_ONLY = 0;
        const WRITE_ONLY = 1 << 0;
        const READ_WRITE = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

/// Create a copy of the opened file by fd
pub fn dup(fd: usize) -> isize {
    sys_dup(fd)
}
/// Open a file with flags
pub fn open(path: &str, flags: OpenFlags) -> isize {
    sys_open(path, flags.bits())
}
/// Close a file
pub fn close(fd: usize) -> isize {
    sys_close(fd)
}
/// Open a pipe for the current process.
/// # Parameter
/// * 'pipe_fd' - The address of a usize array with a length of 2.
///               The kernel will write the file descriptors of the read and write ends of the pipe into the array.
pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    sys_pipe(pipe_fd)
}

/// Read a piece of content from the file into the buffer.
/// # Parameter
/// * 'fd' - file descriptor.
/// * 'buffer' - buffer.
/// # Return
/// * -1 - if read error.
/// * length of bytes read.
pub fn read(fd: usize, buffer: &mut [u8]) -> isize {
    sys_read(fd, buffer)
}
/// Write buffer to file
pub fn write(fd: usize, buffer: &[u8]) -> isize {
    sys_write(fd, buffer)
}
/// Exit a process
pub fn exit(xstate: i32) -> isize {
    sys_exit(xstate)
}
/// yield is the rust key word, so this function named yield_
pub fn yield_() -> isize {
    sys_yield()
}
/// Get cpu execution time
pub fn get_time() -> isize {
    sys_get_time()
}
/// Get current process pid
pub fn get_pid() -> isize {
    sys_get_pid()
}
/// The current process creates a child process.
/// # Return
/// * For the parent process: return the pid of the child process.
/// * For child processes: return 0.
pub fn fork() -> isize {
    sys_fork()
}
/// Clear the address space of the current process and load an executable file, then start execution.
/// # Parameter
/// * 'path' - path to executable file.
/// # Return
/// * -1 if something goes wrong, or no return.
pub fn exec(path: &str, args: &[*const u8]) -> isize {
    sys_exec(path, args)
}
/// Wait for any child process that becomes a zombie process, recycle resources and collect return values.
/// # Parameter
/// * 'exit_code' - child process return value.
/// # Return
/// * -1, If there is no child process.
/// * -2, If all child processes have not ended.
/// * pid, The pid of the child process that ended.
pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut i32) {
            -2 => {
                yield_();
            }
            exit_pid => return exit_pid,
        }
    }
}
/// Wait for specific child process that becomes a zombie process, recycle resources and collect return values.
/// # Parameter
/// * 'pid' - The pid of the child process waiting to end.
/// * 'exit_code' - child process return value.
/// # Return
/// * -1, If there is no child process.
/// * -2, If all child processes have not ended.
/// * pid, The pid of the child process that ended.
pub fn wait_pid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut i32) {
            -2 => {
                yield_();
            }
            exit_pid => return exit_pid,
        }
    }
}
/// Create a child process and run the specified program.
/// # Parameter
/// * 'path' - Path to executable file.
/// # Return
/// * -1 - If error
/// * pid - If success
pub fn spawn(path: &str) -> isize {
    sys_spawn(path)
}

pub fn sleep(time_ms: usize) {
    let start = sys_get_time();
    while sys_get_time() < start + time_ms as isize {
        sys_yield();
    }
}

pub const SIGDEF: i32 = 0; // Default signal handling
pub const SIGHUP: i32 = 1;
pub const SIGINT: i32 = 2;
pub const SIGQUIT: i32 = 3;
pub const SIGILL: i32 = 4;
pub const SIGTRAP: i32 = 5;
pub const SIGABRT: i32 = 6;
pub const SIGBUS: i32 = 7;
pub const SIGFPE: i32 = 8;
pub const SIGKILL: i32 = 9;
pub const SIGUSR1: i32 = 10;
pub const SIGSEGV: i32 = 11;
pub const SIGUSR2: i32 = 12;
pub const SIGPIPE: i32 = 13;
pub const SIGALRM: i32 = 14;
pub const SIGTERM: i32 = 15;
pub const SIGSTKFLT: i32 = 16;
pub const SIGCHLD: i32 = 17;
pub const SIGCONT: i32 = 18;
pub const SIGSTOP: i32 = 19;
pub const SIGTSTP: i32 = 20;
pub const SIGTTIN: i32 = 21;
pub const SIGTTOU: i32 = 22;
pub const SIGURG: i32 = 23;
pub const SIGXCPU: i32 = 24;
pub const SIGXFSZ: i32 = 25;
pub const SIGVTALRM: i32 = 26;
pub const SIGPROF: i32 = 27;
pub const SIGWINCH: i32 = 28;
pub const SIGIO: i32 = 29;
pub const SIGPWR: i32 = 30;
pub const SIGSYS: i32 = 31;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct SignalFlags: u32 {
        const SIGDEF = 1; // Default signal handling
        const SIGHUP = 1 << 1;
        const SIGINT = 1 << 2;
        const SIGQUIT = 1 << 3;
        const SIGILL = 1 << 4;
        const SIGTRAP = 1 << 5;
        const SIGABRT = 1 << 6;
        const SIGBUS = 1 << 7;
        const SIGFPE = 1 << 8;
        const SIGKILL = 1 << 9;
        const SIGUSR1 = 1 << 10;
        const SIGSEGV = 1 << 11;
        const SIGUSR2 = 1 << 12;
        const SIGPIPE = 1 << 13;
        const SIGALRM = 1 << 14;
        const SIGTERM = 1 << 15;
        const SIGSTKFLT = 1 << 16;
        const SIGCHLD = 1 << 17;
        const SIGCONT = 1 << 18;
        const SIGSTOP = 1 << 19;
        const SIGTSTP = 1 << 20;
        const SIGTTIN = 1 << 21;
        const SIGTTOU = 1 << 22;
        const SIGURG = 1 << 23;
        const SIGXCPU = 1 << 24;
        const SIGXFSZ = 1 << 25;
        const SIGVTALRM = 1 << 26;
        const SIGPROF = 1 << 27;
        const SIGWINCH = 1 << 28;
        const SIGIO = 1 << 29;
        const SIGPWR = 1 << 30;
        const SIGSYS = 1 << 31;
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct SignalAction {
    pub handler: usize,
    pub mask: SignalFlags,
}
impl SignalAction {
    pub fn default() -> Self {
        Self {
            handler: 0,
            mask: SignalFlags::empty(),
        }
    }
}

/// Send a signal to a process.
pub fn kill(pid: usize, signum: i32) -> isize {
    sys_kill(pid, signum)
}

/// Set signal handling function for the current process.
pub fn sigaction(
    signum: i32,
    action: Option<&SignalAction>,
    old_action: Option<&mut SignalAction>,
) -> isize {
    sys_sigaction(
        signum,
        action.map_or(core::ptr::null(), |act| act),
        old_action.map_or(core::ptr::null_mut(), |act| act),
    )
}

/// Set the global signal mask for the current process.
pub fn sigprocmask(mask: u32) -> isize {
    sys_sigprocmask(mask)
}

/// Return to process from signal handling function.
pub fn sigreturn() -> isize {
    sys_sigreturn()
}
