use crate::{console::get_char, print, task::suspended_current_and_run_next};

use super::File;

pub struct Stdin;
pub struct Stdout;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }

    fn writable(&self) -> bool {
        false
    }

    fn read(&self, mut buf: crate::mm::page_table::UserBuffer) -> usize {
        assert_eq!(buf.len(), 1);
        let mut c;
        loop {
            c = get_char();
            if c != 0 {
                break;
            }
            suspended_current_and_run_next();
        }
        unsafe {
            buf.buffers[0].as_mut_ptr().write_volatile(c);
        }
        1
    }
    #[allow(unused)]
    fn write(&self, buf: crate::mm::page_table::UserBuffer) -> usize {
        panic!("Can not write to Stdin.")
    }
}

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }

    fn writable(&self) -> bool {
        true
    }

    #[allow(unused)]
    fn read(&self, buf: crate::mm::page_table::UserBuffer) -> usize {
        panic!("Can not read from Stdout.")
    }

    fn write(&self, buf: crate::mm::page_table::UserBuffer) -> usize {
        for slice in buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*slice).unwrap());
        }
        buf.len()
    }
}
