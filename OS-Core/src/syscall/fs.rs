// File stream mod

use alloc::sync::Arc;

use crate::{
    fs::{
        inode::{open_file, OpenFlags},
        pipe::create_pipe,
    },
    mm::page_table::{translate_byte_buffer, PageTable, UserBuffer},
    task::processor::{current_process, current_user_token},
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
    let proc = current_process();
    let inner = proc.inner_exclusive_access();
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
    let proc = current_process();
    let inner = proc.inner_exclusive_access();
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
    let proc = current_process();
    let token = current_user_token();
    let path = PageTable::from_token(token).translated_str(path);
    if let Some(inode) = open_file(&path, OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = proc.inner_exclusive_access();
        let fd = inner.open_file(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let proc = current_process();
    let mut inner = proc.inner_exclusive_access();
    if fd < inner.fd_table.len() && inner.fd_table[fd].is_some() {
        inner.fd_table[fd].take();
        0
    } else {
        -1
    }
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let token = current_user_token();
    let proc = current_process();
    let (read_end, write_end) = create_pipe();
    let mut task_inner = proc.inner_exclusive_access();
    let read_end_fd = task_inner.open_file(read_end);
    let write_end_fd = task_inner.open_file(write_end);
    *PageTable::from_token(token).translated_refmut(pipe) = read_end_fd;
    *PageTable::from_token(token).translated_refmut(unsafe { pipe.add(1) }) = write_end_fd;
    0
}

pub fn sys_dup(fd: usize) -> isize {
    let proc = current_process();
    let mut inner = proc.inner_exclusive_access();
    if fd >= inner.fd_table.len() || inner.fd_table[fd].is_none() {
        return -1;
    }
    let fd_clone = Arc::clone(inner.fd_table[fd].as_ref().unwrap());
    let new_fd = inner.open_file(fd_clone);
    new_fd as isize
}
