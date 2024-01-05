use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::println;

pub fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// Get app binary data from .data section in kernel by app_id
pub fn load_app(app_id: usize) -> &'static [u8] {
    //_num_app:
    //  .quad 5
    //  .quad app_0_start
    //  .quad app_1_start
    //  .quad app_2_start
    //  .quad app_3_start
    //  ......
    //  .quad app_n_start
    //  .quad app_n_end
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}

lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        let num_app = get_num_app();
        extern "C" {
            fn _app_names();
        }
        let mut start = _app_names as usize as *const u8;
        let mut result: Vec<&str> = Vec::new();
        unsafe {
            for _ in 0..num_app {
                let mut end = start;
                while end.read_volatile() != b'\0' {
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let name_str = core::str::from_utf8(slice).unwrap();
                result.push(name_str);
                start = end.add(1);
            }
        }
        result
    };
}

pub fn load_app_from_name(app_name: &str) -> Option<&'static [u8]> {
    (0..get_num_app())
        .find(|&id| APP_NAMES[id] == app_name)
        .map(|id| load_app(id))
}

pub fn list_app() {
    println!("************ APPS ************");
    for name in APP_NAMES.iter() {
        println!("{}", name);
    }
    println!("******************************");
}
