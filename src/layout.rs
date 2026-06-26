use std::sync::atomic::AtomicUsize;

/// Aligns data to the typical L1/L2 cache line size (64 bytes).
/// Prevents false sharing between Producer and Consumer threads.
#[repr(C, align(64))]
pub struct CacheAligned<T>(pub T);

/// Fixed-offset header for the shared memory region.
#[repr(C)]
pub struct SharedHeader {
    /// Write index. Incremented by Producer, read by Consumer.
    pub head: CacheAligned<AtomicUsize>,
    
    /// Read index. Incremented by Consumer, read by Producer.
    pub tail: CacheAligned<AtomicUsize>,
    
    /// Total number of slots. Must be a power of two.
    pub capacity: usize,
    
    /// Size of a single slot in bytes.
    pub item_size: usize,
    
    /// Magic number for initialization checks.
    pub magic: u32,
}

impl SharedHeader {
    pub const MAGIC_VALUE: u32 = 0x58495043; // 'XIPC'
}
