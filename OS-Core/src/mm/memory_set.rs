use super::{
    address::{PhysPageNum, VPNRange, VirtAddr, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
    page_table::{PTEFlags, PageTable, PageTableEntry},
};
use crate::{
    config::{PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USABLE_MEMORY_END, USER_STACK_SIZE},
    mm::address::{PhysAddr, StepByOne},
    println,
    sync::UPSafeCell,
};
use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use core::arch::asm;
use lazy_static::lazy_static;
use riscv::register::satp;

bitflags! {
    pub struct MapPermission:u8{
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

/// How physical pages and virtual pages are mapped
/// * 'Identical' - Identity mapping, ppn==vpn, use for kernel
/// * 'Framed' - Randomly assign physical page frames
#[derive(PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
}

/// A continuous virtual page that has been mapped,
/// has the same mapping method, and has the same permissions.
/// * 'vpn_range' - A contiguous virtual page that can be converted into an iterator.
/// * 'data_frames' - A mapping of virtual pages to physical pages.
/// * 'map_type' - How virtual pages and physical pages are mapped.
/// * 'map_permission' - Permissions for all virtual pages in this area.
pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_permission: MapPermission,
}

impl MapArea {
    /// Create memory_set with the given starting virtual address,
    /// ending virtual address, mapping method and permission.
    /// This memory area must contain the starting address and the ending address,
    /// so the starting virtual page number is the starting virtual address rounded down,
    /// and the ending virtual page number is the ending virtual address rounded up.
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_permission: MapPermission,
    ) -> Self {
        Self {
            vpn_range: VPNRange::new(start_va.floor(), end_va.ceil()),
            data_frames: BTreeMap::new(),
            map_type,
            map_permission,
        }
    }

    /// To associate a virtual page with a physical page,
    /// the specific method is to first apply for a physical page,
    /// then use the map function of pagetable to put the physical page number into the page table entry,
    /// and at the same time save the mapping relationship between the virtual page and the physical page in data_frames.
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let flags = PTEFlags::from_bits(self.map_permission.bits()).unwrap();
        let ppn = match self.map_type {
            MapType::Identical => PhysPageNum(vpn.0),
            MapType::Framed => {
                let fame = frame_alloc().unwrap();
                let temp = fame.ppn;
                self.data_frames.insert(vpn, fame);
                temp
            }
        };
        page_table.map(vpn, ppn, flags);
    }

    /// Map all virtual page in vpn_range,
    /// when the memory area is placed in a MemorySet,
    /// all virtual pages in the memory area will be mapped to real physical pages.
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    /// To cancel the mapping between a virtual page and a physical page,
    /// first delete the page table entry.
    /// If the physical page corresponding to the virtual page is randomly allocated,
    /// the physical page will be recycled at the same time.
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
            _ => {}
        }
        page_table.unmap(vpn);
    }

    /// Unmap all virtual page in vpn_range.
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    /// Copy a piece of data to this memory area.
    /// Note that copying starts from the first page of the memory area.
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let len = data.len();
        let mut src_start = 0usize;
        let mut dst_start = self.vpn_range.get_start();
        loop {
            let src = &data[src_start..len.min(PAGE_SIZE)];
            let dst = &mut page_table
                .translate(dst_start)
                .unwrap()
                .ppn()
                .get_physical_page_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            src_start += PAGE_SIZE;
            if src_start >= len {
                break;
            }
            dst_start.next();
        }
    }
}

/// A program's memory area, including all directory pages and data pages
/// * 'page_table' - All directory pages for program.
/// * 'areas' - All data pages for program.
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    /// Create a MemorySet and allocate only a root directory table and no memory area.
    pub fn new() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }
    /// The highest-addressed virtual page of all MemorySets
    ///     is associated with the physical page starting with strampoline symbol.
    /// The beginning of the physical page starting with the strampoline symbol
    ///     is the assembly code of the entire trap.S file.
    fn map_trampoline(&mut self) {
        extern "C" {
            fn strampoline();
        }
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }
    /// Enable memory paging in sv39 mode.
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            // SFENCE.VMA tells CPU to check page tbl updates
            asm!("sfence.vma");
        }
    }
    /// Put a memory area into the current MemorySet.
    /// If data is not None, data will also be copied to the memory area at the same time.
    fn push(&mut self, map_area: MapArea, data: Option<&[u8]>) {
        let mut map_area = map_area;
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }
    /// Physical pages are allocated by start_va and end_va and inserted into MemorySet
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }
    /// Create a MemorySet for the application through elf,
    /// return the MemorySet, user_sp, and application entry point
    pub fn new_app_from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new();
        memory_set.map_trampoline();
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = (ph.virtual_addr() as usize + ph.mem_size() as usize).into();
                let mut map_permission = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_permission |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_permission |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_permission |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_permission);
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_buttom: usize = max_end_va.into();
        // Empty virtual page for guard
        user_stack_buttom += PAGE_SIZE;
        let user_stack_top = user_stack_buttom + USER_STACK_SIZE;
        memory_set.push(
            MapArea::new(
                user_stack_buttom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        (
            memory_set,
            user_stack_top,
            elf_header.pt2.entry_point() as usize,
        )
    }
    /// Create a kernel MemorySet using identity mapping
    pub fn new_kernel() -> Self {
        extern "C" {
            fn stext(); // begin addr of text segment
            fn etext(); // end addr of text segment
            fn srodata(); // start addr of Read-Only data segment
            fn erodata(); // end addr of Read-Only data ssegment
            fn sdata(); // start addr of data segment
            fn edata(); // end addr of data segment
            fn sbss(); // start addr of BSS segment
            fn ebss(); // end addr of BSS segment
            fn ekernel();
        }
        let mut kernel_memory_set = Self::new();
        kernel_memory_set.map_trampoline();
        println!(
            "[kernel] .text [{:#x}, {:#x})",
            stext as usize, etext as usize
        );
        println!(
            "[kernel] .rodata [{:#x}, {:#x})",
            srodata as usize, erodata as usize
        );
        println!(
            "[kernel] .data [{:#x}, {:#x})",
            sdata as usize, edata as usize
        );
        println!("[kernel] .bss [{:#x}, {:#x})", sbss as usize, ebss as usize);

        println!("[kernel] mapping .text section");
        kernel_memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );

        println!("[kernel] mapping .rodata section");
        kernel_memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );

        println!("[kernel] mapping .data section");
        kernel_memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        println!("[kernel] mapping .bss section");
        kernel_memory_set.push(
            MapArea::new(
                (sbss as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        println!("[kernel] mapping physical memory");
        kernel_memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                USABLE_MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        kernel_memory_set
    }
    /// Find a page table entry by virtual page number.
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }
    /// get token (satp reg) of memory set
    pub fn token(&self) -> usize {
        self.page_table.token()
    }
}

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}
