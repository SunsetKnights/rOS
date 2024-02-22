use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    block_cache::get_block_cache, block_dev::BlockDevice, efs::EasyFileSystem, layout::DiskInode,
};

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
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
}
