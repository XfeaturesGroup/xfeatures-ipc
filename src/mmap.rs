use memmap2::{MmapMut, MmapOptions};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid magic number. Expected {expected:#X}, found {found:#X}")]
    InvalidMagic { expected: u32, found: u32 },
    #[error("Buffer full")]
    BufferFull,
    #[error("Capacity must be a power of two")]
    InvalidCapacity,
    #[error("Configuration mismatch: capacity={capacity}, item_size={item_size}")]
    ConfigMismatch { capacity: usize, item_size: usize },
}

/// OS-agnostic shared memory wrapper.
pub struct SharedMem {
    mmap: MmapMut,
    path: PathBuf,
    is_creator: bool,
}

impl SharedMem {
    /// Creates a new zero-filled memory-mapped file. Fails if the file already exists.
    pub fn create<P: AsRef<Path>>(path: P, size: usize) -> Result<Self, IpcError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path.as_ref())?;

        file.set_len(size as u64)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok(Self {
            mmap,
            path: path.as_ref().to_path_buf(),
            is_creator: true,
        })
    }

    /// Opens an existing memory-mapped file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, IpcError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.as_ref())?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok(Self {
            mmap,
            path: path.as_ref().to_path_buf(),
            is_creator: false,
        })
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.mmap.as_mut_ptr()
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.mmap.as_ptr()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.mmap.len()
    }
}

impl Drop for SharedMem {
    fn drop(&mut self) {
        if self.is_creator {
            // Unlink the file from the filesystem.
            // On Unix, memory persists until all file descriptors are closed.
            // On Windows, sharing violations may prevent deletion. We ignore the error.
            let _ = fs::remove_file(&self.path);
        }
    }
}
