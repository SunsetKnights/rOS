use alloc::vec::Vec;
use easy_fs::block_dev::BlockDevice;
use lazy_static::lazy_static;
use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};

use crate::{
    mm::{
        address::{PhysAddr, PhysPageNum, VirtAddr},
        frame_allocator::{frame_alloc, frame_dealloc, FrameTracker},
        memory_set::kernel_token,
        page_table::PageTable,
    },
    sync::UPSafeCell,
};

lazy_static! {
    static ref QUEUE_FRAMES: UPSafeCell<Vec<FrameTracker>> = unsafe { UPSafeCell::new(Vec::new()) };
}
pub struct VirtIOHal;
impl Hal for VirtIOHal {
    fn dma_alloc(pages: usize) -> virtio_drivers::PhysAddr {
        if pages == 0 {
            return 0;
        }
        let base_page = frame_alloc().unwrap();
        let base_ppn = base_page.ppn.0;
        let base_addr = PhysAddr::from(base_page.ppn).0;
        QUEUE_FRAMES.exclusive_access().push(base_page);
        for i in 1..pages {
            let page = frame_alloc().unwrap();
            assert_eq!(
                page.ppn.0,
                base_ppn + i,
                "VirtQueue requires contiguous physical memory."
            );
            QUEUE_FRAMES.exclusive_access().push(page);
        }
        base_addr
    }

    fn dma_dealloc(paddr: virtio_drivers::PhysAddr, pages: usize) -> i32 {
        let ppn = PhysPageNum::from(paddr).0;
        (ppn..ppn + pages).for_each(|p| frame_dealloc(PhysPageNum(p)));
        0
    }

    /// Only kernel will access deriver.
    fn phys_to_virt(paddr: virtio_drivers::PhysAddr) -> virtio_drivers::VirtAddr {
        paddr
    }

    fn virt_to_phys(vaddr: virtio_drivers::VirtAddr) -> virtio_drivers::PhysAddr {
        //In the kernel address space,
        //most physical addresses and virtual addresses are the same,
        //but the trampoline is mapped at the highest address,
        //so the virtual address cannot be returned directly.
        PageTable::from_token(kernel_token())
            .translate_va(VirtAddr::from(vaddr))
            .0
    }
}

const VIRTIO_0: usize = 0x10001000;
pub struct VitrIOBlock(UPSafeCell<VirtIOBlk<'static, VirtIOHal>>);
impl VitrIOBlock {
    pub fn new() -> Self {
        unsafe {
            Self(UPSafeCell::new(
                VirtIOBlk::<VirtIOHal>::new(&mut *(VIRTIO_0 as *mut VirtIOHeader)).unwrap(),
            ))
        }
    }
}

impl BlockDevice for VitrIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .exclusive_access()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk.");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .exclusive_access()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk.");
    }
}
