use crate::mmap::{IpcError, SharedMem};
use crate::ring_buffer::{Consumer, Producer};
use std::env;
use std::marker::PhantomData;
use std::mem;

/// Builder for configuring and establishing an IPC Ring Buffer connection.
pub struct IpcBuilder<T: Copy> {
    name: String,
    capacity: usize,
    _marker: PhantomData<T>,
}

impl<T: Copy> IpcBuilder<T> {
    /// Creates a new builder for a shared memory segment with the given logical name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            capacity: 256, // Default capacity
            _marker: PhantomData,
        }
    }

    /// Sets the maximum capacity of the ring buffer. Must be a power of two.
    pub fn capacity(mut self, size: usize) -> Self {
        self.capacity = size;
        self
    }

    /// Helper to resolve the correct path based on OS conventions.
    fn resolve_path(&self) -> std::path::PathBuf {
        if cfg!(unix) {
            std::path::PathBuf::from(format!("/dev/shm/xfeatures_ipc_{}", self.name))
        } else {
            env::temp_dir().join(format!("xfeatures_ipc_{}.shm", self.name))
        }
    }

    /// Builds the IPC channel, acting as the Creator.
    /// Returns both the Producer and Consumer halves for same-process usage.
    pub fn build(self) -> Result<(Producer<T>, Consumer<T>), IpcError> {
        let path = self.resolve_path();
        let _ = std::fs::remove_file(&path); // Clean up stale files before creating new one

        let item_size = mem::size_of::<T>();
        let header_size = mem::size_of::<crate::layout::SharedHeader>();
        let mem_size = header_size + self.capacity * item_size;

        let prod_mem = SharedMem::create(&path, mem_size)?;
        let prod = Producer::init(prod_mem, self.capacity)?;

        let cons_mem = SharedMem::open(&path)?;
        let cons = Consumer::new(cons_mem)?;

        Ok((prod, cons))
    }

    /// Builds only the Producer half (Creator of the shared memory).
    pub fn build_producer(self) -> Result<Producer<T>, IpcError> {
        let path = self.resolve_path();
        let _ = std::fs::remove_file(&path);

        let item_size = mem::size_of::<T>();
        let header_size = mem::size_of::<crate::layout::SharedHeader>();
        let mem_size = header_size + self.capacity * item_size;

        let prod_mem = SharedMem::create(&path, mem_size)?;
        Producer::init(prod_mem, self.capacity)
    }

    /// Builds only the Consumer half (Attaches to existing shared memory).
    pub fn build_consumer(self) -> Result<Consumer<T>, IpcError> {
        let path = self.resolve_path();
        let cons_mem = SharedMem::open(&path)?;
        Consumer::new(cons_mem)
    }
}
