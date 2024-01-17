// File stream mod

use crate::{
    console::get_char,
    mm::page_table::translate_byte_buffer,
    print,
    task::{processor::current_user_token, suspended_current_and_run_next},
};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1; // file descriptor 1: standard output stream

/// Wtire buf of lenth "len" to file fd.
/// # parameter
/// * 'fd' - file descriptor
/// * 'buf' - a pointer to the content to be written
/// * 'len' -  the length of the content to be written
/// # return
/// * the length of the fd successfully written
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

/// Read a char to buf from fd.
/// # parameter
/// * 'fd' - file descriptor
/// * 'buf' - a pointer to the content to be written
/// * 'len' -  the length of the content to be written
/// # return
/// * the length of the fd successfully read.
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let mut c;
            loop {
                c = get_char();
                if c != 0 {
                    break;
                }
                suspended_current_and_run_next();
            }
            let mut buffer = translate_byte_buffer(current_user_token(), buf, len);
            unsafe {
                buffer[0].as_mut_ptr().write_volatile(c);
            }
            1
        }
        _ => {
            panic!("unsupported fd in sys_read!");
        }
    }
}
