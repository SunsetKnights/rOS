use alloc::sync::{Arc, Weak};

use crate::{sync::UPSafeCell, task::suspended_current_and_run_next};

use super::File;

const RING_BUFFER_SIZE: usize = 32;
#[derive(PartialEq, Eq)]
enum RingBufferStatus {
    Full,
    Empty,
    Normal,
}

pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    write_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
            write_end: None,
        }
    }

    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }

    pub fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::Normal;
        let byte = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::Empty;
        }
        byte
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.status = RingBufferStatus::Normal;
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }

    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::Empty {
            0
        } else {
            match self.head >= self.tail {
                true => self.tail + RING_BUFFER_SIZE - self.head,
                false => self.tail - self.head,
            }
        }
    }

    pub fn available_write(&self) -> usize {
        if self.status == RingBufferStatus::Full {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }

    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<UPSafeCell<PipeRingBuffer>>,
}

impl Pipe {
    pub fn read_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer,
        }
    }

    pub fn write_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer,
        }
    }
}

impl File for Pipe {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read(&self, buf: crate::mm::page_table::UserBuffer) -> usize {
        assert!(self.readable, "File is not readable.");
        let need_read = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut already_read = 0;
        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let available_read = ring_buffer.available_read();
            if available_read == 0 {
                if ring_buffer.all_write_ends_closed() {
                    return already_read;
                }
                drop(ring_buffer);
                suspended_current_and_run_next();
                continue;
            }
            for _ in 0..available_read {
                if let Some(p) = buf_iter.next() {
                    unsafe { *p = ring_buffer.read_byte() };
                    already_read += 1;
                    if already_read == need_read {
                        return need_read;
                    }
                } else {
                    return already_read;
                }
            }
        }
    }

    fn write(&self, buf: crate::mm::page_table::UserBuffer) -> usize {
        assert!(self.writable, "File is not writable.");
        let need_write = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut already_write = 0;
        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let available_write = ring_buffer.available_write();
            if available_write == 0 {
                drop(ring_buffer);
                suspended_current_and_run_next();
                continue;
            }
            for _ in 0..available_write {
                if let Some(p) = buf_iter.next() {
                    ring_buffer.write_byte(unsafe { *p });
                    already_write += 1;
                    if already_write == need_write {
                        return need_write;
                    }
                } else {
                    return already_write;
                }
            }
        }
    }
}

pub fn create_pipe() -> (Arc<Pipe>, Arc<Pipe>) {
    let buffer = Arc::new(unsafe { UPSafeCell::new(PipeRingBuffer::new()) });
    let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
    let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
    buffer.exclusive_access().set_write_end(&write_end);
    (read_end, write_end)
}
