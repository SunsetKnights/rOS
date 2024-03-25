use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::{
    fs::inode::{open_file, OpenFlags},
    println,
};

use self::{
    manager::add_task,
    processor::{current_task, schedule, take_current_task},
    signal::{SignalFlags, MAX_SIG},
    task::{ProcessControlBlock, TaskStatus},
};

pub mod action;
pub mod context;
pub mod manager;
pub mod pid;
pub mod processor;
pub mod signal;
pub mod switch;
pub mod task;

lazy_static! {
    pub static ref INITPROC: Arc<ProcessControlBlock> = Arc::new(ProcessControlBlock::new(
        open_file("initproc", OpenFlags::READ_ONLY)
            .unwrap()
            .read_all()
            .as_slice()
    ));
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;
    let mut initproc_inner = INITPROC.inner_exclusive_access();
    for child in inner.children.iter() {
        child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
        initproc_inner.children.push(child.clone());
    }
    inner.children.clear();
    drop(initproc_inner);
    inner.memory_set.recycle_data_pages();
    drop(inner);
    schedule();
}

pub fn suspended_current_and_run_next() {
    schedule();
}

pub fn get_pid() -> isize {
    current_task().unwrap().get_pid() as isize
}

pub fn current_add_signal(signal: SignalFlags) {
    let pcb = current_task().unwrap();
    pcb.inner_exclusive_access().signals.insert(signal);
}

pub fn handle_signal() {
    loop {
        check_pending_signal();
        let (frozen, killed) = {
            let task = current_task().unwrap();
            let inner = task.inner_exclusive_access();
            (inner.frozen, inner.killed)
        };
        if !frozen || killed {
            break;
        }
        suspended_current_and_run_next();
    }
}

pub fn check_current_signals_error() -> Option<(i32, &'static str)> {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .signals
        .check_error()
}

pub fn check_pending_signal() {
    let signals = current_task().unwrap().inner_exclusive_access().signals;
    for sig in signals.iter() {
        let task = current_task().unwrap();
        let inner = task.inner_exclusive_access();
        let signum = (0..=MAX_SIG)
            .find(|&x| sig == SignalFlags::from_bits(1 << x).unwrap())
            .unwrap();
        if !inner.signal_mask.contains(sig) {
            let mut masked = true;
            if inner.handling_sig.is_empty() {
                masked = false;
            } else if !inner.signal_actions.table[signum].mask.contains(sig) {
                masked = false;
            }
            if !masked {
                drop(inner);
                drop(task);
                if sig == SignalFlags::SIGKILL
                    || sig == SignalFlags::SIGSTOP
                    || sig == SignalFlags::SIGCONT
                    || sig == SignalFlags::SIGDEF
                {
                    kernel_signal_handler(sig);
                } else {
                    user_signal_handler(signum, sig);
                    return;
                }
            }
        }
    }
}

/// Handle SIGSTOP SIGCONT SIGKILL and SIGDEF signal,
/// SIGTOP and SIGCONT will frozen and unfrozen process.
/// SIGKILL and SIGDEF will kill process.
fn kernel_signal_handler(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    match signal {
        SignalFlags::SIGSTOP => {
            inner.frozen = true;
            inner.signals.remove(signal);
        }
        SignalFlags::SIGCONT => {
            inner.frozen = false;
            inner.signals.remove(signal);
        }
        _ => inner.killed = true,
    }
}

/// Call user signal handle function from signal.
fn user_signal_handler(signum: usize, signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let handler = inner.signal_actions.table[signum].handler;
    if handler != 0 {
        // handle signal flag
        inner.handling_sig = signal;
        inner.signals.remove(signal);
        // backup trap context
        let trap_context_backup = inner.get_trap_context();
        inner.trap_context_backup = Some(*trap_context_backup);
        // modify trap address and set argument for trap handle function
        trap_context_backup.sepc = handler;
        trap_context_backup.x[10] = signal.bits() as usize;
    } else {
        println!(
            "[kernel] The user program does not set a function to handle {}",
            signal.iter_names().next().unwrap().0
        );
    }
}
