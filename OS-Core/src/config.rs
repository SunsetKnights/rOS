// All system constants

pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; //8K
pub const KERNEL_HEAP_SIZE: usize = 0x300000; //3M
pub const MAX_APP_NUM: usize = 5;
pub const APP_BASE_ADDRESS: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x20000;

pub const CLOCK_FREQ: usize = 12500000;
pub const TICKS_PER_SEC: usize = 100;

pub const SYSCALL_QUANTITY: usize = 5;
pub const SYSCALL_ID: [usize; SYSCALL_QUANTITY] = [64, 93, 124, 169, 410];
