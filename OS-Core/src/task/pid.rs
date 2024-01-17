use alloc::vec::Vec;
use core::mem::size_of;
use lazy_static::lazy_static;

use crate::{
    config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE},
    mm::{
        address::VirtAddr,
        memory_set::{MapPermission, KERNEL_SPACE},
    },
    sync::UPSafeCell,
};

trait PidAlloctor {
    fn new() -> Self;
    fn alloc(&mut self) -> PidHandle;
    fn dealloc(&mut self, pid: usize);
}

pub struct SequencePidAllocator {
    current: usize, // current usize
    recycled: Vec<usize>,
}

impl PidAlloctor for SequencePidAllocator {
    /// Create a SequencePidAllocator object
    fn new() -> Self {
        Self {
            current: 0,
            recycled: Vec::new(),
        }
    }
    /// Allocate a pid
    fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            return PidHandle(pid);
        } else {
            let ret = PidHandle(self.current);
            self.current += 1;
            return ret;
        }
    }

    /// Deallocate a physical page, if the page has never been assigned or has been recycled, panic
    fn dealloc(&mut self, pid: usize) {
        if pid > self.current {
            panic!("This pid({}) has never been assigned.", pid);
        }
        if self.recycled.iter().find(|p| **p == pid).is_some() {
            panic!("This pid({}) has been recycled.", pid);
        }
        self.recycled.push(pid);
    }
}

pub struct PidHandle(pub usize);

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR: UPSafeCell<SequencePidAllocator> =
        unsafe { UPSafeCell::new(SequencePidAllocator::new()) };
}

pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.exclusive_access().alloc()
}

/// Get the top and bottom pointers of the process kernel stack through pid.
/// The stack grows from high to low, so bottom > top.
/// # Parameter
/// * 'pid' - process pid.
/// # Return
/// * (stack top pointer, stack bottom pointer)
pub fn kernel_stack_position(pid: usize) -> (usize, usize) {
    let bottom = TRAMPOLINE - pid * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let top = bottom - KERNEL_STACK_SIZE;
    (top, bottom)
}

pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    pub fn new(pid_handle: &PidHandle) -> Self {
        let (top, bottom) = kernel_stack_position(pid_handle.0);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            top.into(),
            bottom.into(),
            MapPermission::R | MapPermission::W,
        );
        Self { pid: pid_handle.0 }
    }

    pub fn push_on_bottom<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let t = (self.get_bottom() - size_of::<T>()) as *mut T;
        unsafe { *t = value };
        t
    }

    pub fn get_bottom(&self) -> usize {
        let (_, bottom) = kernel_stack_position(self.pid);
        bottom
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (top, _) = kernel_stack_position(self.pid);
        KERNEL_SPACE
            .exclusive_access()
            .remove_area_whith_start_vpn(VirtAddr::from(top).into());
    }
}
