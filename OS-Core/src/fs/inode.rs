use alloc::{sync::Arc, vec, vec::Vec};
use easy_fs::{efs::EasyFileSystem, vfs::Inode};
use lazy_static::lazy_static;

use crate::{drivers::block::BLOCK_DEVICE, println, sync::UPSafeCell};

use super::File;

bitflags! {
    pub struct OpenFlags: u32{
        const READ_ONLY = 0;
        const WRITE_ONLY = 1 << 0;
        const READ_WRITE = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRITE_ONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,
}
impl OSInode {
    /// Create a new OSInode from Inode.
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0, inode }) },
        }
    }

    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let len = inner.inode.len() as usize - inner.offset;
        let mut data = vec![0; len];
        let mut slice = data.as_mut_slice();
        inner.inode.read_at(inner.offset, &mut slice);
        inner.offset += len;
        slice.to_vec()
    }
}
impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }
    /// Read data to user buffer from file.
    fn read(&self, mut buf: crate::mm::page_table::UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut read_size = 0;
        for slice in buf.buffers.iter_mut() {
            let curr_read_size = inner.inode.read_at(inner.offset, *slice);
            if curr_read_size == 0 {
                break;
            }
            inner.offset += curr_read_size;
            read_size += curr_read_size;
        }
        read_size
    }
    /// Write data to file from user buffer.
    fn write(&self, buf: crate::mm::page_table::UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut write_size = 0;
        for slice in buf.buffers.iter() {
            let curr_write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(curr_write_size, slice.len(), "Error when writing.");
            inner.offset += curr_write_size;
            write_size += curr_write_size;
        }
        write_size
    }
}

lazy_static! {
    /// Open easy file system and get root inode.
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}

pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            ROOT_INODE
                .create_file(name)
                .map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
        }
    } else {
        ROOT_INODE.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear()
            }
            Arc::new(OSInode::new(readable, writable, inode))
        })
    }
}

pub fn list_app() {
    println!("************ APPS ************");
    for name in ROOT_INODE.list() {
        println!("{}", name);
    }
    println!("******************************");
}
