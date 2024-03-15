use alloc::{string::String, sync::Arc, vec::Vec};
use spin::{Mutex, MutexGuard};

use crate::{
    block_cache::get_block_cache,
    block_dev::BlockDevice,
    efs::EasyFileSystem,
    layout::{DirEntry, DiskInode, DiskInodeType},
    DIRENTRY_SIZE,
};

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a new inode.
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    fn read_disk_inode<V>(&self, func: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, func)
    }

    fn modify_disk_inode<V>(&self, func: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, func)
    }

    /// Find a inode number from directory inode from file or directory name.
    /// # Parameter
    /// * 'name' - File or directory name.
    /// * 'disk_inode' - A Directory inode.
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        assert!(disk_inode.is_directory(), "disk_inode must be directory.");
        let mut dir_entry = DirEntry::empty();
        for i in 0..disk_inode.size as usize / DIRENTRY_SIZE {
            assert_eq!(
                disk_inode.read_at(
                    DIRENTRY_SIZE * i,
                    dir_entry.as_bytes_mut(),
                    &self.block_device
                ),
                DIRENTRY_SIZE,
                "Faild to read directory entry."
            );
            if dir_entry.get_name() == name {
                return Some(dir_entry.get_inode_number());
            }
        }
        None
    }

    // Just for debug
    // pub fn inode_num(&self, file_name: &str) -> Option<u32> {
    //     let _fs = self.fs.lock();
    //     self.read_disk_inode(|disk_inode| self.find_inode_id(file_name, disk_inode))
    // }

    /// Get inode length.
    pub fn len(&self) -> u32 {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.size)
    }

    /// Get inode position in device.
    pub fn device_position(&self) -> (usize, usize) {
        (self.block_id, self.block_offset)
    }

    /// Get the inode of a file or directory by its name in the current directory inode.
    pub fn find(&self, file_name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(file_name, disk_inode).map(|inode_id| {
                let (block_id, offset) = fs.inode_position(inode_id);
                Arc::new(Self::new(
                    block_id,
                    offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    /// Increase inode size.
    /// # Parameter
    /// * 'new_size' - New size after increase.
    /// * 'disk_inode' - DiskInode corresponding to the current Inode.
    /// * 'fs' - EFS that got lock.
    pub fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size > disk_inode.size {
            let block_needed_quantity = disk_inode.blocks_num_needed(new_size);
            let mut needed_blocks = Vec::with_capacity(block_needed_quantity as usize);
            for _ in 0..block_needed_quantity {
                needed_blocks.push(fs.alloc_data());
            }
            disk_inode.increase_size(new_size, needed_blocks, &self.block_device);
        }
    }

    /// Create a new inode in current directory.
    /// # Parameter
    /// * 'name' - File or directory name.
    /// * 'type_' - Inode type (file or directory).
    /// # Return
    /// * None if inode exist, or Arc<Inode>
    fn create(&self, name: &str, type_: DiskInodeType) -> Option<Arc<Inode>> {
        // If file name already exist, then return None.
        if self.find(name).is_some() {
            return None;
        }
        let mut fs = self.fs.lock();
        // Alloc and initialize new inode.
        let new_inode_id = fs.alloc_inode();
        let (new_inode_block_id, new_block_offset) = fs.inode_position(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(type_);
            });
        // Add DirEntry to current inode.
        self.modify_disk_inode(|directory| {
            // Increase current inode size.
            let inode_count = directory.size as usize / DIRENTRY_SIZE;
            let new_size = directory.size + DIRENTRY_SIZE as u32;
            self.increase_size(new_size, directory, &mut fs);
            // Add new DirEntry
            let direntry = DirEntry::new(name, new_inode_id);
            directory.write_at(
                inode_count * DIRENTRY_SIZE,
                direntry.as_bytes(),
                &self.block_device,
            );
        });
        let result = Self::new(
            new_inode_block_id,
            new_block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        );
        Some(Arc::new(result))
    }

    pub fn create_file(&self, name: &str) -> Option<Arc<Inode>> {
        self.create(name, DiskInodeType::File)
    }

    pub fn create_directory(&self, name: &str) -> Option<Arc<Inode>> {
        self.create(name, DiskInodeType::Directory)
    }

    /// Recycle all index blocks and data blocks from inode.
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let recycle_size = disk_inode.size;
            let dealloc_blocks = disk_inode.clear_size(&self.block_device);
            assert_eq!(
                dealloc_blocks.len(),
                DiskInode::total_blocks_by_size(recycle_size) as usize
            );
            for block in dealloc_blocks {
                fs.dealloc_data(block);
            }
        });
    }

    pub fn read_at(&self, offset: usize, buffer: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buffer, &self.block_device))
    }

    pub fn write_at(&self, offset: usize, buffer: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buffer.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buffer, &self.block_device)
        })
    }

    /// Get all file names in the current folder.
    pub fn list(&self) -> Vec<String> {
        // Get lock
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_quantity = disk_inode.size as usize / DIRENTRY_SIZE;
            let mut result = Vec::with_capacity(file_quantity);
            for i in 0..file_quantity {
                let mut direntry = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(
                        i * DIRENTRY_SIZE,
                        direntry.as_bytes_mut(),
                        &self.block_device
                    ),
                    DIRENTRY_SIZE,
                    "Faild to read directory entry."
                );
                result.push(String::from(direntry.get_name()));
            }
            result
        })
    }
}
