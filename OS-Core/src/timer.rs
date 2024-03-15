use crate::config::TICKS_PER_SEC;
use crate::platfrom::CLOCK_FREQ;
use crate::sbi_services::set_timer;
use riscv::register::time;

pub fn get_time() -> usize {
    time::read()
}

pub fn get_time_ms() -> usize {
    time::read() / CLOCK_FREQ * 1000
}

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC)
}
