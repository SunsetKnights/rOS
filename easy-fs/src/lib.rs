#![no_std]
extern crate alloc;
pub mod bitmap;
pub mod block_cache;
pub mod block_dev;
pub mod efs;
pub mod layout;
pub mod vfs;

pub const BLOCK_SIZE: usize = 512;
pub const MAX_BLOCK_CACHE_QUANTITY: usize = 16;
pub const EFS_MAGIC: u32 = 7604003;
pub const INODE_DIRECT_COUNT: usize = 28;
pub const INODE_INDIRECT_1_COUNT: usize = BLOCK_SIZE / 4;
pub const INDIRECT_1_BOUND: usize = INODE_DIRECT_COUNT + INODE_INDIRECT_1_COUNT;
pub const NAME_LENGTH_LIMIT: usize = 27;
pub const DIRENTRY_SIZE: usize = 32;
