use super::{
    address::{PhysPageNum, StepByOne, VirtAddr, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
};
use crate::config::SV39_PPN_WIDTH;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
    #[derive(PartialEq, Eq)]
    pub struct PTEFlags:u8{
        const V = 1 << 0;   // Valid
        const R = 1 << 1;   // Read
        const W = 1 << 2;   // Write
        const X = 1 << 3;   // Execute
        const U = 1 << 4;   // User access
        const G = 1 << 5;   // ???
        const A = 1 << 6;   // Accessed
        const D = 1 << 7;   // Dirty
    }
}

///Page table entry for directory page.
/// Bit location like: 53,52,51......3,2,1,0.
///# PTE struct:
///* [0-7]: V R W X U G A D
///* [8-9]: RSW, Ignored by MMU
///* [10-53]: Physical page number

#[derive(Clone, Copy)]
#[repr(C)]
pub struct PageTableEntry {
    pub pte: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            pte: ppn.0 << 10 | flags.bits() as usize,
        }
    }
    pub fn empty() -> Self {
        PageTableEntry { pte: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {
        PhysPageNum(self.pte >> 10 & ((1 << SV39_PPN_WIDTH) - 1))
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.pte as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

pub struct PageTable {
    // Root physical page for application program
    root_ppn: PhysPageNum,
    // All physical page frame for application program
    frames: Vec<FrameTracker>,
}

impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// When accessing the memory of the user program,
    /// the root page table address is found through satp,
    /// and then the address is translated through the root page table
    pub fn from_token(token: usize) -> Self {
        Self {
            root_ppn: PhysPageNum(token & ((1 << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Find the page table entry of the physical page through the virtual page number.
    /// If some of the directory pages are empty, create a new directory page.
    ///
    /// # Parameter
    /// * 'vpn' - Need to find the virtual page number of the page table entry.
    fn find_pte_or_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let offsets = vpn.get_offsets();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[offsets[i]];
            if i == 2 {
                result = Some(pte);
                break;
            }
            // In fact, if the page table entry does not exist: pte.pte==0, pte.is_valid()==false
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find the page table entry of the physical page through the virtual page number.
    /// If some of the directory pages are empty, return None.
    ///
    /// # Parameter
    /// * 'vpn' - Need to find the virtual page number of the page table entry.
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let offsets = vpn.get_offsets();
        let mut ppn = self.root_ppn;
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[offsets[i]];
            if i == 2 {
                return Some(pte);
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        None
    }
    /// To associate a virtual page and a physical page,
    /// first find the page table entry through vpn,
    /// then set the address part of the page table entry to ppn,
    /// set the flags part to flags, and set the v flag to 1.
    ///
    /// # Parameter
    /// * 'vpn' - The associated virtual page number.
    /// * 'ppn' - The associated physical page number.
    /// * 'flags' - Page table entry flags.
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_or_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping.", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    /// To cancel the mapping between the physical page number and the virtual page number,
    /// only need to reset the ppn corresponding to the virtual page to zero.
    ///
    /// # Parameter
    /// * 'vpn' - Virtual page number that needs to be unmapped.
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping.", vpn);
        *pte = PageTableEntry::empty();
    }
    /// Find a page table entry by virtual page number.
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// Construct satp register contents through paging mode and root physical page address.
    /// paging mode (high 4 bits):
    ///     8: Sv39
    ///     9: Sv48
    ///     10: Sv57
    ///     11: Sv64
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// Read data from user memory set.
///
/// # Parameter
/// * 'token' - satp of user memory set
/// * 'user_va' - virtual address of user memory set
/// * 'len' - data length
/// # Return
/// * Data slicing of user memory space data in kernel address space
pub fn translate_byte_buffer(token: usize, user_va: *const u8, len: usize) -> Vec<&'static [u8]> {
    let user_page_table = PageTable::from_token(token);
    let mut start = user_va as usize;
    let end = start + len;
    let mut ret: Vec<&[u8]> = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = user_page_table.translate(vpn).unwrap().ppn();
        // end_va is the starting address of the next page, or an address of this page
        vpn.next();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));

        if end_va.page_offset() == 0 {
            ret.push(&ppn.get_physical_page_bytes_array()[start_va.page_offset()..]);
        } else {
            ret.push(
                &ppn.get_physical_page_bytes_array()[start_va.page_offset()..end_va.page_offset()],
            );
        }
        start = end_va.into();
    }
    ret
}
