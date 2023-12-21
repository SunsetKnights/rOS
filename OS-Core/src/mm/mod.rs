use self::memory_set::KERNEL_SPACE;

pub mod address;
pub mod frame_allocator;
pub mod heap_allocator;
pub mod memory_set;
pub mod page_table;

pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::frame_allocator_init();
    KERNEL_SPACE.exclusive_access().activate();
}
