//! A contiguous (virtual) memory, circular queue that uses raw pointers to read/write
//!
//! It is lock-free and wait-free, unless the buffer is full
//!
//! The idea is to deploy this queue in shared memory for single producer single consumer
//! SPSC communication

use std::{
    io, ptr,
    sync::atomic::{AtomicUsize, Ordering},
};

use libc::c_void;

#[repr(align(64))]
pub struct RingBuf<T> {
    capacity: isize,
    base: *mut T,
    write: AtomicUsize,
    read: AtomicUsize,
}

impl<T> RingBuf<T> {
    /// Create a new RingBuf
    /// Provide a base pointer to base the queue
    /// and a capacity indicating the size of the queue
    /// The size of the queue is 1 less than the provided capacity
    pub fn new(bs: *mut c_void, cap: usize) -> Self {
        let base = bs as *mut T;
        let write = AtomicUsize::new(0);
        let read = AtomicUsize::new(0);
        let capacity = cap as isize;
        Self {
            capacity,
            base,
            write,
            read,
        }
    }

    /// Get the read and write pointers from the queue as u8
    pub fn pointers(&self) -> [u8; 2] {
        [
            self.write.load(Ordering::Relaxed) as u8,
            self.read.load(Ordering::Relaxed) as u8,
        ]
    }

    /// Push an element into the queue
    /// Increments the write pointer
    pub fn push(&self, elem: T) -> io::Result<()> {
        unsafe {
            let mut cur_ind = self.write.load(Ordering::Relaxed) as isize;
            cur_ind = (cur_ind + 1) % self.capacity;
            if cur_ind == self.read.load(Ordering::Acquire) as isize {
                return Err(io::Error::new(io::ErrorKind::Other, "Buf full"));
            }
            self.write.store(cur_ind as usize, Ordering::Release);
            std::ptr::write(self.base.offset(cur_ind), elem);
        }
        Ok(())
    }

    /// Pop an element from the queue
    /// Increments the read pointer
    pub fn pop(&self) -> Option<T> {
        unsafe {
            let mut cur_ind = self.read.load(Ordering::Relaxed) as isize;
            if cur_ind == self.write.load(Ordering::Acquire) as isize {
                return None;
            }
            cur_ind = (cur_ind + 1) % self.capacity;
            self.read.store(cur_ind as usize, Ordering::Release);
            let elem = ptr::read(self.base.offset(cur_ind));
            Some(elem)
        }
    }
}
