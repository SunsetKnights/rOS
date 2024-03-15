// File stream mod

use crate::{
    fs::inode::{open_file, OpenFlags},
    mm::page_table::{translate_byte_buffer, PageTable, UserBuffer},
    task::processor::{current_task, current_user_token},
};

/// Wtire buf of lenth "len" to file fd.
/// # parameter
/// * 'fd' - file descriptor
/// * 'buf' - a pointer to the content to be written
/// * 'len' -  the length of the content to be written
/// # return
/// * the length of the fd successfully written
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = inner.fd_table[fd].clone() {
        drop(inner);
        file.write(UserBuffer::new(translate_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

/// Read a char to buf from fd.
/// # parameter
/// * 'fd' - file descriptor
/// * 'buf' - a pointer to the content to be written
/// * 'len' -  the length of the content to be written
/// # return
/// * the length of the fd successfully read.
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = inner.fd_table[fd].clone() {
        drop(inner);
        file.read(UserBuffer::new(translate_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = PageTable::from_token(token).translated_str(path);
    if let Some(inode) = open_file(&path, OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd < inner.fd_table.len() && inner.fd_table[fd].is_some() {
        inner.fd_table[fd].take();
        0
    } else {
        -1
    }
}
