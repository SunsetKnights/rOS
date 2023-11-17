// File stream mod

use crate::print;

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
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            len as isize
        }
        _ => {
            panic!("unsupported fd in sys_write!");
        }
    }
}
