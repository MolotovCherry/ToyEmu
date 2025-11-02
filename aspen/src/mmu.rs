mod address_range;
mod memory;

use std::sync::atomic::{AtomicU8, Ordering};

use enumflags2::{BitFlag, BitFlags, bitflags};

use crate::{
    BitSize,
    mmu::{
        address_range::AddressRange,
        memory::{FromBytes, ToBytes},
    },
};
use memory::Memory;

pub type Protection = BitFlags<Prot>;

pub const MEM_SIZE: usize = BitSize::MAX as usize + 1;
pub const PAGE_SIZE: usize = {
    let size = 4096;
    assert!(MEM_SIZE.is_multiple_of(size));
    size
};

macro_rules! page_idx {
    ($addr:ident) => {{ ($addr / PAGE_SIZE as u32) as usize }};
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum MemError {
    #[error("Page fault: {0} access denied")]
    PageFault(Protection),
    #[error("Overflow occurred")]
    Overflow,
    #[cfg(windows)]
    #[error("Winapi Error: {0}")]
    WinApi(#[from] windows::core::Error),
    #[cfg(windows)]
    #[error("Alloc failed: {0:?}")]
    Alloc(windows::Win32::Foundation::WIN32_ERROR),
    #[cfg(unix)]
    #[error("I/O Error: {0}")]
    Io(std::sync::Arc<std::io::Error>),
}

/// Protection state of page
#[rustfmt::skip]
#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Prot {
    Read    = 0b001,
    Write   = 0b010,
    Execute = 0b100,
}

#[derive(Default, Debug)]
struct Page {
    prot: AtomicU8,
}

impl Page {
    fn prot(&self) -> Protection {
        // SAFETY: this is the proper type (otherwise typecheck fails),
        // and the value stored here was created from Prot::bits()
        unsafe { Prot::from_bits_unchecked(self.prot.load(Ordering::Relaxed)) }
    }

    fn set_prot(&self, prot: Protection) {
        self.prot.store(prot.bits(), Ordering::Relaxed);
    }
}

#[derive(Debug)]
pub struct Mmu {
    pages: Vec<Page>,
    mem: Memory,
}

impl Mmu {
    pub fn new() -> Result<Self, MemError> {
        let mut pages = Vec::with_capacity(MEM_SIZE / PAGE_SIZE);
        for _ in 0..pages.capacity() {
            pages.push(Page::default());
        }

        let this = Self {
            pages,
            mem: Memory::new()?,
        };

        Ok(this)
    }

    /// Get the page belonging to addr
    pub fn prot(&self, addr: BitSize) -> Protection {
        let idx = page_idx!(addr);
        self.pages[idx].prot()
    }

    /// Change memory protection for a page.
    /// Note: All page(s) covering the range are changed
    pub fn set_prot(&self, addr: impl Into<AddressRange>, prot: impl Into<Protection>) {
        let prot = prot.into();
        let addr = addr.into().into_iter();

        for addr in addr.step_by(PAGE_SIZE) {
            let idx = page_idx!(addr);
            self.pages[idx].set_prot(prot);
        }
    }

    /// Check whether all pages in address range are a particular Protection
    /// Fails if any pages do not meet the condition
    pub fn check_prot(
        &self,
        addr: impl Into<AddressRange>,
        req: impl Into<Protection>,
    ) -> Result<(), MemError> {
        let req = req.into();
        let addr = addr.into().into_iter();

        for addr in addr.step_by(PAGE_SIZE) {
            let idx = page_idx!(addr);
            let record = self.pages[idx].prot();
            if !record.contains(req) {
                let i = !record & req;
                return Err(MemError::PageFault(i));
            }
        }

        Ok(())
    }

    /// Access raw mem
    ///
    /// # Safety
    /// No read or writes of any kind are allowed while this slice is alive
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn mem(&self) -> &[u8; MEM_SIZE] {
        unsafe { self.mem.mem() }
    }

    /// Access raw mutable mem
    ///
    /// # Safety
    /// No read or writes of any kind are allowed while this slice is alive
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn mem_mut(&self) -> &mut [u8; MEM_SIZE] {
        unsafe { self.mem.mem_mut() }
    }

    /// Copy mem to buffer starting at addr
    pub fn memcpy(&self, addr: BitSize, buf: &mut [u8]) -> Result<(), MemError> {
        self.mem.memcpy(addr, buf)
    }

    /// Write buffer to memory starting at addr
    pub fn memwrite(&self, addr: BitSize, buf: &[u8]) -> Result<(), MemError> {
        self.mem.memwrite(addr, buf)
    }

    /// Read, but don't check protection
    pub fn read_unchecked<N: FromBytes>(&self, addr: BitSize) -> Result<N, MemError> {
        let n = self.mem.read(addr)?;
        Ok(n)
    }

    // Write, but don't check protection
    pub fn write_unchecked<N: Copy + ToBytes>(&self, addr: BitSize, n: N) -> Result<(), MemError> {
        self.mem.write(addr, n)?;
        Ok(())
    }

    // Read with protection check
    pub fn read<N: FromBytes>(&self, addr: BitSize) -> Result<N, MemError> {
        self.check_prot(addr, Prot::Read)?;
        let n = self.mem.read(addr)?;
        Ok(n)
    }

    /// Write with protection check
    pub fn write<N: Copy + ToBytes>(&self, addr: BitSize, n: N) -> Result<(), MemError> {
        self.check_prot(addr, Prot::Write)?;
        self.mem.write(addr, n)?;
        Ok(())
    }

    /// Zeroes memory
    ///
    /// # Safety
    /// This function cannot be called while any views exist or read/write happen
    pub unsafe fn zeroize(&self) -> Result<(), MemError> {
        unsafe { self.mem.zeroize() }
    }
}
