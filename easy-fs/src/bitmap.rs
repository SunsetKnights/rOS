use alloc::sync::Arc;

use crate::{block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_SIZE};

const BLOCK_BITS: usize = BLOCK_SIZE * 8;
type BitmapBlock = [u64; BLOCK_BITS / 64];

pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}

impl Bitmap {
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    /// Returns the first free block number.
    /// This block number is the Bitmap internal block number,
    /// not the device block number.
    /// # Parameter
    /// * 'block_device' - Block device.
    /// # Return
    /// * If free block exist, return first free block number, or return None.
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {
            let free_block_position =
                get_block_cache(self.start_block_id + block_id, Arc::clone(block_device))
                    .lock()
                    .modify(0, |bitmap_block: &mut BitmapBlock| {
                        if let Some((bitmap_postion, inner_postion)) = bitmap_block
                            .iter()
                            .enumerate()
                            .find(|(_, bit64)| **bit64 != u64::MAX)
                            .map(|(bitmap_postion, bit64)| {
                                (bitmap_postion, bit64.trailing_ones() as usize)
                            })
                        {
                            bitmap_block[bitmap_postion] |= 1 << inner_postion;
                            Some(block_id * BLOCK_BITS + bitmap_postion * 64 + inner_postion)
                        } else {
                            None
                        }
                    });
            if free_block_position.is_some() {
                return free_block_position;
            }
        }
        None
    }

    /// Dealloc a block by Bitmap internal number.
    /// # Parameter
    /// * 'block_device' - Block device.
    /// * 'bit_position' - Block id in Bitmap.
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit_position: usize) {
        let block_position = bit_position / BLOCK_BITS;
        let block_inner_position = bit_position % BLOCK_BITS;
        let bitmap_position = block_inner_position / 64;
        let inner_position = block_inner_position % 64;
        get_block_cache(
            self.start_block_id + block_position,
            Arc::clone(block_device),
        )
        .lock()
        .modify(0, |bitmap_block: &mut BitmapBlock| {
            assert!(bitmap_block[bitmap_position] & (1 << inner_position) > 0);
            bitmap_block[bitmap_position] &= !(1 << inner_position);
        })
    }

    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}
