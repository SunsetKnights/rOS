use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{block_dev::BlockDevice, BLOCK_SIZE, MAX_BLOCK_CACHE_QUANTITY};

pub struct BlockCache {
    cache: [u8; BLOCK_SIZE],
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
    modified: bool,
}

impl BlockCache {
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    fn addr_of_offset(&self, offest: usize) -> usize {
        &self.cache[offest] as *const _ as usize
    }

    pub fn get_ref<T>(&self, offest: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offest + type_size <= BLOCK_SIZE);
        let addr = self.addr_of_offset(offest);
        unsafe { &*(addr as *const T) }
    }

    pub fn read<T, V>(&self, offest: usize, func: impl FnOnce(&T) -> V) -> V {
        func(self.get_ref(offest))
    }

    pub fn get_mut<T>(&mut self, offest: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offest + type_size <= BLOCK_SIZE);
        self.modified = true;
        let addr = self.addr_of_offset(offest);
        unsafe { &mut *(addr as *mut T) }
    }

    pub fn modify<T, V>(&mut self, offest: usize, func: impl FnOnce(&mut T) -> V) -> V {
        func(self.get_mut(offest))
    }

    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache);
        }
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync();
    }
}

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some((_, cache)) = self.queue.iter().find(|(id, _)| *id == block_id) {
            Arc::clone(cache)
        } else {
            if self.queue.len() == MAX_BLOCK_CACHE_QUANTITY {
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, (_, cache))| Arc::strong_count(cache) == 1)
                {
                    self.queue.remove(idx);
                } else {
                    panic!("Run out of BlockCache");
                }
            }
            let block_cache = Arc::new(Mutex::new(BlockCache::new(block_id, block_device)));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static! {
    pub static ref BLOCK_CACHE_MAMAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}

pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MAMAGER
        .lock()
        .get_block_cache(block_id, block_device)
}
