use crate::layout::SharedHeader;
use crate::mmap::{IpcError, SharedMem};
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::sync::atomic::Ordering;

/// SPSC Ring Buffer Producer.
pub struct Producer<T: Copy> {
    shared_mem: SharedMem,
    capacity: usize,
    mask: usize,
    data_ptr: *mut u8,
    cached_tail: usize,
    _marker: PhantomData<T>,
}

/// SPSC Ring Buffer Consumer.
pub struct Consumer<T: Copy> {
    shared_mem: SharedMem,
    capacity: usize,
    mask: usize,
    data_ptr: *const u8,
    cached_head: usize,
    _marker: PhantomData<T>,
}

#[inline(always)]
fn is_power_of_two(n: usize) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

impl<T: Copy> Producer<T> {
    pub fn init(mut shared_mem: SharedMem, capacity: usize) -> Result<Self, IpcError> {
        if !is_power_of_two(capacity) {
            return Err(IpcError::InvalidCapacity);
        }

        let item_size = mem::size_of::<T>();
        let header_size = mem::size_of::<SharedHeader>();
        let required_size = header_size + capacity * item_size;

        if shared_mem.len() < required_size {
            return Err(IpcError::ConfigMismatch { capacity, item_size });
        }

        let header = unsafe { &mut *(shared_mem.as_mut_ptr() as *mut SharedHeader) };

        header.head.0.store(0, Ordering::Relaxed);
        header.tail.0.store(0, Ordering::Relaxed);
        header.capacity = capacity;
        header.item_size = item_size;
        header.magic = SharedHeader::MAGIC_VALUE;

        let mask = capacity - 1;
        let data_ptr = unsafe { shared_mem.as_mut_ptr().add(header_size) };

        Ok(Self {
            shared_mem,
            capacity,
            mask,
            data_ptr,
            cached_tail: 0,
            _marker: PhantomData,
        })
    }

    pub fn push(&mut self, value: T) -> Result<(), IpcError> {
        let header = unsafe { &*(self.shared_mem.as_ptr() as *const SharedHeader) };

        let current_head = header.head.0.load(Ordering::Relaxed);

        if current_head.wrapping_sub(self.cached_tail) >= self.capacity {
            self.cached_tail = header.tail.0.load(Ordering::Acquire);

            if current_head.wrapping_sub(self.cached_tail) >= self.capacity {
                return Err(IpcError::BufferFull);
            }
        }

        let index = current_head & self.mask;
        let item_size = mem::size_of::<T>();

        unsafe {
            // Calculate pointer based on raw bytes, then cast to generic type T
            let slot_ptr = self.data_ptr.add(index * item_size) as *mut T;
            ptr::write_unaligned(slot_ptr, value);
        }

        header.head.0.store(current_head.wrapping_add(1), Ordering::Release);

        Ok(())
    }
}

impl<T: Copy> Consumer<T> {
    pub fn new(shared_mem: SharedMem) -> Result<Self, IpcError> {
        let header = unsafe { &*(shared_mem.as_ptr() as *const SharedHeader) };

        if header.magic != SharedHeader::MAGIC_VALUE {
            return Err(IpcError::InvalidMagic {
                expected: SharedHeader::MAGIC_VALUE,
                found: header.magic,
            });
        }
        if !is_power_of_two(header.capacity) {
            return Err(IpcError::InvalidCapacity);
        }
        
        let item_size = mem::size_of::<T>();
        if header.item_size != item_size {
            return Err(IpcError::ConfigMismatch {
                capacity: header.capacity,
                item_size: header.item_size,
            });
        }

        let capacity = header.capacity;
        let mask = capacity - 1;
        let header_size = mem::size_of::<SharedHeader>();
        let data_ptr = unsafe { shared_mem.as_ptr().add(header_size) };

        Ok(Self {
            shared_mem,
            capacity,
            mask,
            data_ptr,
            cached_head: 0,
            _marker: PhantomData,
        })
    }

    pub fn pop(&mut self) -> Option<T> {
        let header = unsafe { &*(self.shared_mem.as_ptr() as *const SharedHeader) };

        let current_tail = header.tail.0.load(Ordering::Relaxed);

        if self.cached_head == current_tail {
            self.cached_head = header.head.0.load(Ordering::Acquire);

            if self.cached_head == current_tail {
                return None;
            }
        }

        let index = current_tail & self.mask;
        let item_size = mem::size_of::<T>();
        let value: T;

        unsafe {
            let slot_ptr = self.data_ptr.add(index * item_size) as *const T;
            value = ptr::read_unaligned(slot_ptr);
        }

        header.tail.0.store(current_tail.wrapping_add(1), Ordering::Release);

        Some(value)
    }
}

// SAFETY: SPSC design ensures exclusive write access to respective atomics.
// Shared memory pointers outlive the Producer struct. Data races are prevented via Acquire/Release memory fences.
// Trait T is constrained to Copy, ensuring data is trivially copiable without hidden heap pointers.
unsafe impl<T: Copy> Send for Producer<T> {}

// SAFETY: Producer only mutates memory through atomic operations and safe pointer offsets.
unsafe impl<T: Copy> Sync for Producer<T> {}

// SAFETY: Same as Producer. SPSC exclusivity and atomic fencing guarantees safety.
unsafe impl<T: Copy> Send for Consumer<T> {}

// SAFETY: Consumer only mutates its own tail index via atomic operations.
unsafe impl<T: Copy> Sync for Consumer<T> {}

impl<T: Copy> Drop for Producer<T> {
    fn drop(&mut self) {
        // PANIC SAFETY: If push() panics before updating `head`, the data may be partially copied
        // but it will never be visible to the Consumer. The buffer remains consistent.
    }
}

impl<T: Copy> Drop for Consumer<T> {
    fn drop(&mut self) {
        // PANIC SAFETY: If pop() panics before updating `tail`, the slot remains occupied.
        // It will eventually exhaust capacity, but no memory corruption occurs.
    }
}
