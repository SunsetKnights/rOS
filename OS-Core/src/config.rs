// All system constants

pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2; //8K
pub const KERNEL_HEAP_SIZE: usize = 0x300000; //3M
pub const MAX_APP_NUM: usize = 5;
pub const APP_BASE_ADDRESS: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x20000;

pub const CLOCK_FREQ: usize = 12500000;
pub const TICKS_PER_SEC: usize = 100;

pub const PAGE_SIZE_BITS: usize = 12; // How many bits does it take to access a memory page
pub const PAGE_SIZE: usize = 4096; //0b1000000000000
pub const USABLE_MEMORY_END: usize = 0x80800000;
pub const SV39_PA_WIDTH: usize = 56;
pub const SV39_PPN_WIDTH: usize = SV39_PA_WIDTH - PAGE_SIZE_BITS;
pub const SV39_VA_WIDTH: usize = 39;
pub const SV39_VPN_WIDTH: usize = SV39_VA_WIDTH - PAGE_SIZE_BITS;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

pub const SYSCALL_QUANTITY: usize = 5;
pub const SYSCALL_ID: [usize; SYSCALL_QUANTITY] = [64, 93, 124, 169, 410];
