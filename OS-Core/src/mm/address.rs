use core::fmt::Debug;

use crate::config::{
    PAGE_SIZE, PAGE_SIZE_BITS, SV39_PA_WIDTH, SV39_PPN_WIDTH, SV39_VA_WIDTH, SV39_VPN_WIDTH,
};

use super::page_table::PageTableEntry;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub usize);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysPageNum(pub usize);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtPageNum(pub usize);

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("PA: {:#x}", self.0))
    }
}
impl Debug for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("VA: {:#x}", self.0))
    }
}
impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("PPN: {:#x}", self.0))
    }
}
impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("VPN: {:#x}", self.0))
    }
}

// method for convert usize and physics address
impl From<usize> for PhysAddr {
    fn from(value: usize) -> Self {
        Self(value & ((1 << SV39_PA_WIDTH) - 1)) //only use low 56 bit
    }
}
impl From<PhysAddr> for usize {
    fn from(value: PhysAddr) -> Self {
        value.0
    }
}
// method for convert usize and physics page number
impl From<usize> for PhysPageNum {
    fn from(value: usize) -> Self {
        Self(value & ((1 << SV39_PPN_WIDTH) - 1)) //only use low 44 bit
    }
}
impl From<PhysPageNum> for usize {
    fn from(value: PhysPageNum) -> Self {
        value.0
    }
}
// method for convert usize and virtual address
impl From<usize> for VirtAddr {
    fn from(value: usize) -> Self {
        Self(value & ((1 << SV39_VA_WIDTH) - 1)) //only use low 39 bit
    }
}
impl From<VirtAddr> for usize {
    fn from(value: VirtAddr) -> Self {
        value.0
    }
}
// method for convert usize and virtual page number
impl From<usize> for VirtPageNum {
    fn from(value: usize) -> Self {
        Self(value & ((1 << SV39_VPN_WIDTH) - 1)) //only use low 27 bit
    }
}
impl From<VirtPageNum> for usize {
    fn from(value: VirtPageNum) -> Self {
        value.0
    }
}
// method for convert physics address and physics page number
impl From<PhysAddr> for PhysPageNum {
    fn from(value: PhysAddr) -> Self {
        assert_eq!(value.page_offset(), 0);
        value.floor()
    }
}
impl From<PhysPageNum> for PhysAddr {
    fn from(value: PhysPageNum) -> Self {
        PhysAddr(value.0 << PAGE_SIZE_BITS)
    }
}
// method for convert virtual address and virtual page number
impl From<VirtAddr> for VirtPageNum {
    fn from(value: VirtAddr) -> Self {
        assert_eq!(value.page_offset(), 0);
        value.floor()
    }
}
impl From<VirtPageNum> for VirtAddr {
    fn from(value: VirtPageNum) -> Self {
        VirtAddr(value.0 << PAGE_SIZE_BITS)
    }
}

impl PhysAddr {
    /// Get the page offset of the current physical address.
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    /// Get the page number of the current physical address, rounded down.
    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 >> PAGE_SIZE_BITS)
    }
    /// Get the page number of the current pjysical address, rounded up.
    pub fn ceil(&self) -> PhysPageNum {
        PhysPageNum((self.0 + PAGE_SIZE - 1) >> PAGE_SIZE_BITS)
    }

    pub fn get_mut<T>(&self) -> &'static mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }
}
impl VirtAddr {
    /// Get the page offset of the current virtual address.
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    /// Get the page number of the current virtual address, rounded down.
    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 >> PAGE_SIZE_BITS)
    }
    /// Get the page number of the current virtual address, rounded up.
    pub fn ceil(&self) -> VirtPageNum {
        VirtPageNum((self.0 + PAGE_SIZE - 1) >> PAGE_SIZE_BITS)
    }
}

impl PhysPageNum {
    /// Get a u8 slice of physical page size.
    pub fn get_physical_page_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = self.clone().into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, PAGE_SIZE) }
    }
    /// Get a PageTableEntry slice from physical page,
    /// if the physical page is not a directory page,
    /// this function is meaningless.
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) }
    }
    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = (*self).into();
        unsafe { (pa.0 as *mut T).as_mut().unwrap() }
    }
}

impl VirtPageNum {
    /// Get three offsets into the virtual page number
    pub fn get_offsets(&self) -> [usize; 3] {
        let mut all_offset = self.0;
        let mut offsets = [0usize; 3];
        for i in (0..3).rev() {
            offsets[i] = all_offset & 511;
            all_offset >>= 9;
        }
        offsets
    }
}

///
pub trait StepByOne {
    fn next(&mut self);
}

impl StepByOne for VirtPageNum {
    fn next(&mut self) {
        self.0 += 1;
    }
}

#[derive(Clone, Copy)]
pub struct EasyRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    start: T,
    end: T,
}

impl<T> EasyRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start: {:?} > end: {:?}", start, end);
        Self { start, end }
    }
    pub fn get_start(&self) -> T {
        self.start
    }
    pub fn get_end(&self) -> T {
        self.end
    }
}

impl<T> IntoIterator for EasyRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;

    type IntoIter = EasyRangeIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        EasyRangeIterator::new(self.start, self.end)
    }
}

pub struct EasyRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}

impl<T> EasyRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        Self {
            current: start,
            end,
        }
    }
}

impl<T> Iterator for EasyRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.next();
            Some(t)
        }
    }
}

pub type VPNRange = EasyRange<VirtPageNum>;
