pub mod inode;
mod stdio;
use crate::mm::page_table::UserBuffer;
pub use stdio::{Stdin, Stdout};
pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}
