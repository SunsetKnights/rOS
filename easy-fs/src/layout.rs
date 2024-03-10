use alloc::{sync::Arc, vec::Vec};

use crate::{
    block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_SIZE, DIRENTRY_SIZE, EFS_MAGIC,
    INDIRECT_1_BOUND, INODE_DIRECT_COUNT, INODE_INDIRECT_1_COUNT, NAME_LENGTH_LIMIT,
};

#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_blocks: u32,
    pub inode_bitmap_blocks: u32,
    pub inode_area_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_area_blocks: u32,
}

impl SuperBlock {
    pub fn initialize(
        &mut self,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
        inode_area_blocks: u32,
        data_bitmap_blocks: u32,
        data_area_blocks: u32,
    ) {
        *self = Self {
            magic: EFS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        };
    }

    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}

#[repr(C)]
pub struct DiskInode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect_1: u32,
    pub indirect_2: u32,
    type_: DiskInodeType,
}

type IndirectBlock = [u32; INODE_INDIRECT_1_COUNT];
type DataBlock = [u8; BLOCK_SIZE];

impl DiskInode {
    pub fn initialize(&mut self, type_: DiskInodeType) {
        self.size = 0;
        self.direct.iter_mut().for_each(|v| *v = 0);
        self.indirect_1 = 0;
        self.indirect_2 = 0;
        self.type_ = type_;
    }
    /// Determine whether the current Inode is a directory.
    pub fn is_directory(&self) -> bool {
        self.type_ == DiskInodeType::Directory
    }
    /// Determine whether the current Inode is a file.
    pub fn is_file(&self) -> bool {
        self.type_ == DiskInodeType::File
    }
    /// Get actual block number by the linear data block number inside the Inode.
    /// # Parameter
    /// * 'inner_id' - Linear data block number inside the Inode.
    /// * 'block_device' - Block device driver.
    /// # Return
    /// * Actual block number.
    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let inner_id = inner_id as usize;
        if inner_id < INODE_DIRECT_COUNT {
            self.direct[inner_id]
        } else if inner_id < INDIRECT_1_BOUND {
            get_block_cache(self.indirect_1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indrect_block: &IndirectBlock| {
                    indrect_block[inner_id - INODE_DIRECT_COUNT]
                })
        } else {
            let tail = inner_id - INDIRECT_1_BOUND;
            let indirect_1 = get_block_cache(self.indirect_2 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[tail / INODE_INDIRECT_1_COUNT]
                });
            get_block_cache(indirect_1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[tail % INODE_INDIRECT_1_COUNT]
                })
        }
    }
    /// Calculate the number of data blocks contained in the current Inode.
    /// # Return
    /// * Number of data blocks.
    pub fn data_blocks(&self) -> u32 {
        Self::data_blocks_by_size(self.size)
    }
    /// Calculate the number of blocks required to save size bytes.
    fn data_blocks_by_size(size: u32) -> u32 {
        // rounded up
        (size + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32
    }
    /// Calculate the number of blocks required to save data (including data blocks and index blocks).
    /// # Parameter
    /// * 'size' - Number of bytes of data.
    /// # Return
    /// * Number of blocks.
    pub fn total_blocks_by_size(size: u32) -> u32 {
        let data_blocks = Self::data_blocks_by_size(size);
        let mut total = data_blocks;
        if data_blocks > INODE_DIRECT_COUNT as u32 {
            // indirect1
            total += 1;
        }
        if data_blocks > INDIRECT_1_BOUND as u32 {
            // indirect2
            total += 1;
            // rounded up
            total += (data_blocks - INDIRECT_1_BOUND as u32 + INODE_INDIRECT_1_COUNT as u32 - 1)
                / INODE_INDIRECT_1_COUNT as u32;
        }
        total
    }
    /// Calculate the number of blocks required to increase the Inode to its new size.
    /// # Parameter
    /// * 'new_size' - New size of Inode.
    /// # Return
    /// * Number of blocks that needed.
    pub fn blocks_num_needed(&self, new_size: u32) -> u32 {
        assert!(new_size > self.size);
        Self::total_blocks_by_size(new_size) - Self::total_blocks_by_size(self.size)
    }
    /// Add new blocks to the Inode.
    /// The new blocks must include data blocks and index blocks.
    /// # Parameter
    /// * 'new_size' - New size of Inode.
    /// * 'new_blocks' - The new block that the Inode will contain (including data blocks and index blocks).
    /// * 'block_device' - Block device driver.
    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        let mut current_block = self.data_blocks();
        self.size = new_size;
        let mut total_blocks = self.data_blocks();
        let mut new_blocks = new_blocks.into_iter();
        // fill direct blocks
        while current_block < total_blocks.min(INODE_DIRECT_COUNT as u32) {
            self.direct[current_block as usize] = new_blocks.next().unwrap();
            current_block += 1;
        }
        // A block is required for indirect1.
        // If indirect1 does not exist before, use a newly requested block.
        if total_blocks > INODE_DIRECT_COUNT as u32 {
            if current_block == INODE_DIRECT_COUNT as u32 {
                self.indirect_1 = new_blocks.next().unwrap();
            }
            current_block -= INODE_DIRECT_COUNT as u32;
            total_blocks -= INODE_DIRECT_COUNT as u32;
            // Fill the data block number into the indirect1 block
            get_block_cache(self.indirect_1 as usize, Arc::clone(block_device))
                .lock()
                .modify(0, |indirect_1_block: &mut IndirectBlock| {
                    while current_block < total_blocks.min(INODE_INDIRECT_1_COUNT as u32) {
                        indirect_1_block[current_block as usize] = new_blocks.next().unwrap();
                        current_block += 1;
                    }
                });
        }
        if total_blocks > INODE_INDIRECT_1_COUNT as u32 {
            // Alloc indirect2 block
            if current_block == INODE_INDIRECT_1_COUNT as u32 {
                self.indirect_2 = new_blocks.next().unwrap();
            }
            current_block -= INODE_INDIRECT_1_COUNT as u32;
            total_blocks -= INODE_INDIRECT_1_COUNT as u32;
            get_block_cache(self.indirect_2 as usize, Arc::clone(block_device))
                .lock()
                .modify(0, |indirect_2_block: &mut IndirectBlock| {
                    let mut indirect_1_idx = current_block as usize / INODE_INDIRECT_1_COUNT;
                    let mut inner_idx = current_block as usize % INODE_INDIRECT_1_COUNT;
                    while current_block < total_blocks {
                        // Alloc indirect1 block
                        if inner_idx == 0 {
                            indirect_2_block[indirect_1_idx] = new_blocks.next().unwrap();
                        }
                        get_block_cache(
                            indirect_2_block[indirect_1_idx] as usize,
                            Arc::clone(block_device),
                        )
                        .lock()
                        .modify(
                            0,
                            |indirect_1_block: &mut IndirectBlock| {
                                while current_block < total_blocks
                                    && inner_idx < INODE_INDIRECT_1_COUNT
                                {
                                    // Alloc data block
                                    indirect_1_block[indirect_1_idx] = new_blocks.next().unwrap();
                                    inner_idx += 1;
                                    current_block += 1;
                                }
                                inner_idx = 0;
                            },
                        );
                        indirect_1_idx += 1;
                    }
                });
        }
        // All new blocks should be allocated
        assert!(new_blocks.next().is_none());
    }
    /// Clear the Inode and return all blocks that need to be recycled.
    /// # Parameter
    /// * 'block_device' - Block device driver.
    /// # Return
    /// * All blocks that need to be recycled.
    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        let total_recycle = Self::total_blocks_by_size(self.size);
        let mut data_recycle = self.data_blocks();
        let mut collector: Vec<u32> = Vec::with_capacity(total_recycle as usize);
        // Recycle direct block.
        let mut current_data_block = 0;
        while current_data_block < data_recycle.min(INODE_DIRECT_COUNT as u32) {
            collector.push(self.direct[current_data_block as usize]);
            current_data_block += 1;
        }
        // Recycle indirect1 block.
        if data_recycle > INODE_DIRECT_COUNT as u32 {
            collector.push(self.indirect_1);
            current_data_block -= INODE_DIRECT_COUNT as u32;
            data_recycle -= INODE_DIRECT_COUNT as u32;
            get_block_cache(self.indirect_1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    while current_data_block < data_recycle.min(INODE_INDIRECT_1_COUNT as u32) {
                        collector.push(indirect_block[current_data_block as usize]);
                        current_data_block += 1;
                    }
                });
        }
        // Recycle indirect2 block
        if data_recycle > INODE_INDIRECT_1_COUNT as u32 {
            collector.push(self.indirect_2);
            current_data_block -= INODE_INDIRECT_1_COUNT as u32;
            data_recycle -= INODE_INDIRECT_1_COUNT as u32;
            get_block_cache(self.indirect_2 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect_2_block: &IndirectBlock| {
                    let mut indirect_1_block_idx = 0;
                    while current_data_block < data_recycle {
                        // Recycle indirect1 block
                        collector.push(indirect_2_block[indirect_1_block_idx]);
                        get_block_cache(
                            indirect_2_block[indirect_1_block_idx] as usize,
                            Arc::clone(block_device),
                        )
                        .lock()
                        .read(0, |indirect_1_block: &IndirectBlock| {
                            let mut inner_idx = 0;
                            while current_data_block < data_recycle
                                && inner_idx < INODE_INDIRECT_1_COUNT
                            {
                                // Recycle data block
                                collector.push(indirect_1_block[inner_idx]);
                                inner_idx += 1;
                                current_data_block += 1;
                            }
                        });
                        indirect_1_block_idx += 1;
                    }
                });
        }
        self.size = 0;
        assert_eq!(
            collector.len(),
            total_recycle as usize,
            "The number of recycled blocks is not equal to the total number of Inode blocks."
        );
        collector
    }
    /// Read bytes from file into buffer.
    /// # Parameter
    /// * 'offset' - File offset
    /// * 'buffer' - Buffer in memory
    /// * 'block_device' - Block device.
    /// # Return
    /// * Length of bytes read successfully.
    pub fn read_at(
        &self,
        offest: usize,
        buffer: &mut [u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        // Start offset in file
        let mut start = offest;
        // End offset in file
        let end = (self.size as usize).min(start + buffer.len());
        if end <= start {
            return 0;
        }
        let mut read_size = 0;
        let mut buffer_offset = 0;
        loop {
            let curr_block = start / BLOCK_SIZE;
            let inner_start = start % BLOCK_SIZE;
            let inner_end = match start + BLOCK_SIZE > end {
                true => end % BLOCK_SIZE,
                false => BLOCK_SIZE,
            };
            let curr_len = inner_end - inner_start;
            get_block_cache(
                self.get_block_id(curr_block as u32, block_device) as usize,
                Arc::clone(block_device),
            )
            .lock()
            .read(0, |data_block: &DataBlock| {
                let src = data_block[inner_start..inner_end].as_ptr();
                let dst = buffer[buffer_offset..buffer_offset + curr_len].as_mut_ptr();
                unsafe { dst.copy_from(src, curr_len) };
            });
            read_size += curr_len;
            buffer_offset += curr_len;
            start += curr_len;
            if start == end {
                break;
            }
        }
        read_size
    }
    /// Write bytes from file into buffer.
    /// # Parameter
    /// * 'offset' - File offset
    /// * 'buffer' - Buffer in memory
    /// * 'block_device' - Block device.
    /// # Return
    /// * Length of bytes write successfully.
    pub fn write_at(
        &mut self,
        offest: usize,
        buffer: &[u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        assert!(
            offest + buffer.len() <= self.size as usize,
            "The file length is too small and cannot be written to the buffer"
        );
        // Start offset in file
        let mut start = offest;
        // End offset in file
        let end = start + buffer.len();
        if end <= start {
            return 0;
        }
        let mut write_size = 0;
        let mut buffer_offset = 0;
        loop {
            let curr_block = start / BLOCK_SIZE;
            let inner_start = start % BLOCK_SIZE;
            let inner_end = match start + BLOCK_SIZE > end {
                true => end % BLOCK_SIZE,
                false => BLOCK_SIZE,
            };
            let curr_len = inner_end - inner_start;
            get_block_cache(
                self.get_block_id(curr_block as u32, block_device) as usize,
                Arc::clone(block_device),
            )
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                let src = buffer[buffer_offset..buffer_offset + curr_len].as_ptr();
                let dst = data_block[inner_start..inner_end].as_mut_ptr();
                unsafe { dst.copy_from(src, curr_len) };
            });
            write_size += curr_len;
            buffer_offset += curr_len;
            start += curr_len;
            if start == end {
                break;
            }
        }
        write_size
    }
}

#[repr(C)]
pub struct DirEntry {
    name: [u8; NAME_LENGTH_LIMIT + 1], // The end must be '\0'.
    inode_number: u32,
}

impl DirEntry {
    /// Create a empty directory entry.
    pub fn empty() -> Self {
        Self {
            name: [0; NAME_LENGTH_LIMIT + 1],
            inode_number: 0,
        }
    }

    /// Create a new directory entry from file name or directory name
    /// and inode number.
    /// # Parameter
    /// * 'name' - File or directory name, max length is NAME_LENGTH_LIMIT+1, end with '\0'
    /// * 'inode_number' - File or directory inode number.
    /// # Return
    /// * A directory entry.
    pub fn new(name: &str, inode_number: u32) -> Self {
        assert!(
            name.len() < NAME_LENGTH_LIMIT + 1,
            "File or directory name is too long."
        );
        let mut name_arr = [0; NAME_LENGTH_LIMIT + 1];
        (&mut name_arr[..name.len()]).copy_from_slice(name.as_bytes());
        Self {
            name: name_arr,
            inode_number,
        }
    }

    /// Convert a directory entry to u8 slice.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self as *const _ as usize as *const u8, DIRENTRY_SIZE)
        }
    }

    /// Convert a directory entry to mutable u8 slice.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self as *mut _ as usize as *mut u8, DIRENTRY_SIZE)
        }
    }

    /// Get file or directory name, not include '\0'.
    pub fn get_name(&self) -> &str {
        let len = (0..NAME_LENGTH_LIMIT + 1)
            .find(|i| self.name[*i] == 0)
            .unwrap();
        core::str::from_utf8(&self.name[..len]).unwrap()
    }

    /// Get inode number.
    pub fn get_inode_number(&self) -> u32 {
        self.inode_number
    }
}
