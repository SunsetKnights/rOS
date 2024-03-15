mod virtio_blk;
pub use virtio_blk::VitrIOBlock;

use alloc::sync::Arc;
use easy_fs::block_dev::BlockDevice;
use lazy_static::lazy_static;

use crate::platfrom::BlockDeviceImpl;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}
