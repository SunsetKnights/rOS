use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    bitmap::Bitmap,
    block_cache::{get_block_cache, sync_all_block},
    block_dev::BlockDevice,
    layout::{DiskInode, DiskInodeType, SuperBlock},
    vfs::Inode,
    BLOCK_SIZE,
};

pub struct EasyFileSystem {
    pub block_device: Arc<dyn BlockDevice>,
    pub inode_bitmap: Bitmap,
    pub data_bitmap: Bitmap,
    inode_block_start: u32,
    data_block_start: u32,
}

type DataBlock = [u8; BLOCK_SIZE];

impl EasyFileSystem {
    /// Create an EasyFileSystem.
    /// # Parameter
    /// * 'total_blocks' - Total number of disk blocks.
    /// * 'inode_bitmap_blocks' - The number of blocks occupied by inode bitmap.
    /// # Return
    /// * A new EasyFileSystem
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        // Block 0 is the super block, so inode bitmap block start with block 1
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
        let inode_quantity = inode_bitmap.maximum();
        // ceil
        let inode_blocks_quantity =
            ((inode_quantity * core::mem::size_of::<DiskInode>() + BLOCK_SIZE - 1) / BLOCK_SIZE)
                as u32;
        let inode_total_blocks = inode_bitmap_blocks + inode_blocks_quantity;
        let data_total_blocks = total_blocks - 1 - inode_total_blocks;
        // ceil
        let data_bitmap_blocks = (data_total_blocks + 4096) / 4097;
        let data_blocks_quantity = data_total_blocks - data_bitmap_blocks;
        let data_bitmap = Bitmap::new(
            (inode_total_blocks + 1) as usize,
            data_bitmap_blocks as usize,
        );
        let mut efs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_block_start: inode_bitmap_blocks + 1,
            data_block_start: inode_total_blocks + data_bitmap_blocks + 1,
        };
        for i in 0..total_blocks as usize {
            get_block_cache(i, Arc::clone(&block_device)).lock().modify(
                0,
                |data_block: &mut DataBlock| {
                    for byte in data_block {
                        *byte = 0;
                    }
                },
            )
        }
        // write the super block
        get_block_cache(0, Arc::clone(&block_device)).lock().modify(
            0,
            |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_blocks_quantity,
                    data_bitmap_blocks,
                    data_blocks_quantity,
                );
            },
        );
        // create root directory
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_block_id, root_inode_block_offset) = efs.inode_position(0);
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_block_offset, |inode: &mut DiskInode| {
                inode.initialize(DiskInodeType::Directory);
            });
        sync_all_block();
        Arc::new(Mutex::new(efs))
    }

    /// Read the necessary information of the EasyFileSystem from disk block 0.
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Super block is valid.");
                let efs = Self {
                    block_device: Arc::clone(&block_device),
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        1 + super_block.inode_bitmap_blocks as usize
                            + super_block.inode_area_blocks as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_block_start: 1 + super_block.inode_bitmap_blocks,
                    data_block_start: 1
                        + super_block.inode_bitmap_blocks
                        + super_block.inode_area_blocks
                        + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }

    /// Alloc a inode (The first free bitmap number).
    /// # Return
    /// * a number representing the inode.
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    /// Alloc a block for data.
    /// # Return
    /// * Disk block id.
    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_block_start
    }

    /// Clear the disk block and set the bitmap of the block to 0.
    /// # Parameter
    /// * 'block_id' - Disk block id.
    pub fn dealloc_data(&mut self, block_id: u32) {
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|b| *b = 0);
            });
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_block_start) as usize,
        );
    }

    /// Get the block number and offset of the block where the inode is located.
    /// # Parameter
    /// * 'inode_id' - Inode id.
    /// # Return
    /// * (block id, offset)
    pub fn inode_position(&self, inode_id: u32) -> (u32, usize) {
        let inode_size = core::mem::size_of::<DiskInode>();
        let block_id = inode_id / (BLOCK_SIZE / inode_size) as u32 + self.inode_block_start;
        let offset = (inode_id as usize % (BLOCK_SIZE / inode_size)) * inode_size;
        (block_id, offset)
    }

    /// Get disk block id from data block id.
    pub fn data_block_id(&self, data_block_id: u32) -> u32 {
        self.data_block_start + data_block_id
    }

    /// Get root inode.
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = Arc::clone(&efs.lock().block_device);
        let (block_id, offset) = efs.lock().inode_position(0);
        Inode::new(block_id, offset, Arc::clone(efs), block_device)
    }
}
