use alloc::vec::Vec;
use lazy_static::lazy_static;

use crate::{config::USABLE_MEMORY_END, info, mm::address::PhysAddr, sync::UPSafeCell};

use super::address::PhysPageNum;

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

pub struct StackFrameAllocator {
    current: usize, // current free physics page number
    end: usize,     // last free physics page number
    recycled: Vec<PhysPageNum>,
}
impl StackFrameAllocator {
    /// Init a StackFrameAllocator from start physical page num and end physical page num
    pub fn init(&mut self, s: PhysPageNum, e: PhysPageNum) {
        self.current = s.0;
        self.end = e.0;
        info!(
            "Usable page number start = {}, end = {}, total {} pages.",
            s.0,
            e.0,
            e.0 - s.0
        );
    }
}

impl FrameAllocator for StackFrameAllocator {
    /// Create a StackFrameAllocator object, call init function before use
    fn new() -> Self {
        StackFrameAllocator {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }
    /// Allocate a physical page, if there is a physical page free, return Some(PhysPageNum), or return None
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if self.recycled.is_empty() {
            if self.current <= self.end {
                let ret = Some(self.current.into());
                self.current += 1;
                return ret;
            } else {
                return None;
            }
        } else {
            return self.recycled.pop();
        }
    }

    /// Deallocate a physical page, if the page has never been assigned or has been recycled, panic
    fn dealloc(&mut self, ppn: PhysPageNum) {
        if ppn.0 > self.current {
            panic!("This page({}) has never been assigned.", ppn.0);
        }
        if self.recycled.iter().find(|p| p.0 == ppn.0).is_some() {
            panic!("This page({}) has been recycled.", ppn.0);
        }
        self.recycled.push(ppn);
    }
}

pub struct FrameTracker {
    pub ppn: PhysPageNum,
}
impl FrameTracker {
    /// Clear a physical page and bind the life cycle of the physical page to Self
    pub fn new(ppn: PhysPageNum) -> Self {
        let page = ppn.get_physical_page_bytes_array();
        page.fill(0);
        FrameTracker { ppn }
    }
}
impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

// Global physical page allocator.
lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<StackFrameAllocator> =
        unsafe { UPSafeCell::new(StackFrameAllocator::new()) };
}

pub fn frame_allocator_init() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(USABLE_MEMORY_END - 1).floor(),
    );
}

pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(|ppn| FrameTracker::new(ppn))
}

pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}
