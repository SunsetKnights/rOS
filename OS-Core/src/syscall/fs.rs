// File stream mod

use crate::{mm::page_table::translate_byte_buffer, print, task::current_user_token};

// file descriptor 1: standard output stream
const FD_STDOUT: usize = 1;

/// wtire buf of lenth "len" to file fd
///
/// # parameter
/// * 'fd' - file descriptor
/// * 'buf' - a pointer to the content to be written
/// * 'len' -  the length of the content to be written
///
/// # return
/// the length of the fd successfully written
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let slice_vec = translate_byte_buffer(current_user_token(), buf, len);
            for slice in slice_vec {
                print!("{}", core::str::from_utf8(slice).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("unsupported fd in sys_write!");
        }
    }
}
