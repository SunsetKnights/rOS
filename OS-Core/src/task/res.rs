use alloc::{
    collections::BTreeSet,
    sync::{Arc, Weak},
};
use core::mem::size_of;
use lazy_static::lazy_static;

use crate::{
    config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT_BASE, USER_STACK_SIZE},
    mm::{
        address::{PhysPageNum, VirtAddr, VirtPageNum},
        memory_set::{MapPermission, KERNEL_SPACE},
    },
    sync::UPSafeCell,
};

use super::process::ProcessControlBlock;

/// Id(usize) allocator.
pub trait IdAlloctor {
    fn new() -> Self;
    fn alloc(&mut self) -> usize;
    fn dealloc(&mut self, id: usize);
}

pub struct SequenceAllocator {
    current: usize,
    recycled: BTreeSet<usize>,
}
impl IdAlloctor for SequenceAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            recycled: BTreeSet::new(),
        }
    }
    fn alloc(&mut self) -> usize {
        if let Some(recycle) = self.recycled.pop_first() {
            recycle
        } else {
            self.current += 1;
            self.current - 1
        }
    }
    fn dealloc(&mut self, id: usize) {
        assert!(id < self.current);
        assert!(
            !self.recycled.contains(&id),
            "id {} has been deallocated!",
            id
        );
        self.recycled.insert(id);
    }
}

type PidAllocator = SequenceAllocator;
type KernelStackAllocator = SequenceAllocator;

lazy_static! {
    /// Global pid allocator.
    static ref PID_ALLOCATOR: UPSafeCell<PidAllocator> =
        unsafe { UPSafeCell::new(PidAllocator::new()) };
    /// Global KernelStack allocator.
    static ref KERNEL_STACK_ALLOCATOR: UPSafeCell<KernelStackAllocator> =
        unsafe { UPSafeCell::new(KernelStackAllocator::new()) };
}
/// RAII style pid encapsulation.
pub struct PidHandle(pub usize);
impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}
/// Alloc a new pid.
pub fn pid_alloc() -> PidHandle {
    PidHandle(PID_ALLOCATOR.exclusive_access().alloc())
}

/// RAII style KernelStack encapsulation.
pub struct KernelStack(pub usize);
/// Get the top and bottom pointers of the thread kernel stack through id.
/// The stack grows from high to low, so bottom > top.
/// # Parameter
/// * 'id' - stack id.
/// # Return
/// * (stack top pointer, stack bottom pointer)
pub fn kernel_stack_position(id: usize) -> (usize, usize) {
    let bottom = TRAMPOLINE - id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let top = bottom - KERNEL_STACK_SIZE;
    (top, bottom)
}
impl Drop for KernelStack {
    /// Unmap kernel stack page, dealloc stack id.
    fn drop(&mut self) {
        let (top, _) = kernel_stack_position(self.0);
        KERNEL_SPACE
            .exclusive_access()
            .remove_area_whith_start_vpn(VirtAddr::from(top).into());
        KERNEL_STACK_ALLOCATOR.exclusive_access().dealloc(self.0)
    }
}
impl KernelStack {
    pub fn push_on_bottom<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let t = (self.get_bottom() - size_of::<T>()) as *mut T;
        unsafe { *t = value };
        t
    }

    pub fn get_bottom(&self) -> usize {
        let (_, bottom) = kernel_stack_position(self.0);
        bottom
    }
}
/// Alloc a new kernel stack
pub fn kernel_stack_alloc() -> KernelStack {
    let stack_id = KERNEL_STACK_ALLOCATOR.exclusive_access().alloc();
    let (top, bottom) = kernel_stack_position(stack_id);
    KERNEL_SPACE.exclusive_access().insert_framed_area(
        top.into(),
        bottom.into(),
        MapPermission::R | MapPermission::W,
    );
    KernelStack(stack_id)
}

pub struct ThreadUserRes {
    pub tid: usize,
    pub user_stack_bottom: usize,
    pub process: Weak<ProcessControlBlock>,
}

fn trap_context_from_tid(tid: usize) -> usize {
    TRAP_CONTEXT_BASE - tid * PAGE_SIZE
}
fn user_stack_top_from_tid(user_stack_base: usize, tid: usize) -> usize {
    user_stack_base + tid * (USER_STACK_SIZE + PAGE_SIZE)
}

impl ThreadUserRes {
    /// Create a new thread user resource, include tid user stack and trap page.
    pub fn new(user_stack_base: usize, process: Arc<ProcessControlBlock>, alloc_res: bool) -> Self {
        let tid = process.inner_exclusive_access().alloc_tid();
        let user_stack_bottom = user_stack_top_from_tid(user_stack_base, tid) + USER_STACK_SIZE;
        let user_res = Self {
            tid,
            user_stack_bottom,
            process: Arc::downgrade(&process),
        };
        if alloc_res {
            user_res.alloc_res();
        }
        user_res
    }

    /// Map thread user stack and trap page.
    pub fn alloc_res(&self) {
        let proc = self.process.upgrade().unwrap();
        let mut proc_inner = proc.inner_exclusive_access();
        // alloc thread user stack
        proc_inner.memory_set.insert_framed_area(
            (self.user_stack_bottom - USER_STACK_SIZE).into(),
            self.user_stack_bottom.into(),
            MapPermission::R | MapPermission::W | MapPermission::U,
        );
        // alloc trap context page
        let trap_page_start = trap_context_from_tid(self.tid);
        proc_inner.memory_set.insert_framed_area(
            trap_page_start.into(),
            (trap_page_start + PAGE_SIZE).into(),
            MapPermission::R | MapPermission::W,
        );
    }

    /// Dealloc tid, unmap user stack and trap page.
    pub fn dealloc_res(&self) {
        let proc = self.process.upgrade().unwrap();
        let mut proc_inner = proc.inner_exclusive_access();
        // dealloc tid
        proc_inner.dealloc_tid(self.tid);
        // dealloc thread user stack
        let stack_top = self.user_stack_bottom - USER_STACK_SIZE;
        proc_inner
            .memory_set
            .remove_area_whith_start_vpn(stack_top.into());
        // dealloc trap context page
        let trap_page_start = trap_context_from_tid(self.tid);
        proc_inner
            .memory_set
            .remove_area_whith_start_vpn(trap_page_start.into());
    }

    /// Get trap context physical page number.
    pub fn trap_context_ppn(&self) -> PhysPageNum {
        let proc = self.process.upgrade().unwrap();
        let proc_inner = proc.inner_exclusive_access();
        let vpn = VirtPageNum::from(VirtAddr::from(trap_context_from_tid(self.tid)));
        proc_inner.memory_set.translate(vpn).unwrap().ppn()
    }

    /// When the user stack base is changed, the stack bottom position is recalculated.
    pub fn rebase(&mut self, new_user_stack_base: usize) {
        self.user_stack_bottom =
            user_stack_top_from_tid(new_user_stack_base, self.tid) + USER_STACK_SIZE;
    }

    pub fn trap_context_va(&self) -> usize {
        trap_context_from_tid(self.tid)
    }

    pub fn user_stack_bottom(&self) -> usize {
        self.user_stack_bottom
    }
}

impl Drop for ThreadUserRes {
    fn drop(&mut self) {
        self.dealloc_res();
    }
}
