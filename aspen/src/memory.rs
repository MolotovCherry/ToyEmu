use std::{
    ffi::c_void,
    ops::{Bound, RangeBounds},
};

use windows::Win32::{
    Foundation::{GetLastError, WIN32_ERROR},
    System::Memory::{
        MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc, VirtualFree,
    },
};

use crate::BitSize;

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum MemError {
    #[error("Invalid access: size {0} @ 0x{1:08x}")]
    InvalidAddr(BitSize, BitSize),
    #[error("Alloc failed: {0:?}")]
    Alloc(WIN32_ERROR),
}

const MEM_SIZE: usize = BitSize::MAX as usize;

pub struct Memory {
    data: &'static mut [u8; MEM_SIZE],
}

unsafe impl Send for Memory {}

impl Memory {
    pub fn new() -> Result<Self, MemError> {
        #[rustfmt::skip]
        let alloc = unsafe {
            VirtualAlloc(
                None,
                MEM_SIZE,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };

        if alloc.is_null() {
            let err = unsafe { GetLastError() };
            return Err(MemError::Alloc(err));
        }

        // SAFETY:
        // alloc is BitSize::MAX big (above)
        // we also already checked for a failed call
        let data = unsafe { &mut *alloc.cast::<[u8; MEM_SIZE]>() };
        let this = Self { data };

        Ok(this)
    }

    /// Write to an address. Fails if addr+val is out of bounds
    pub fn write<N: Copy + ToBytes>(&mut self, addr: BitSize, val: N) -> Result<(), MemError> {
        self.validate_addr(const { size_of::<N>() as BitSize }, addr)?;

        let buf = self.view_mut(addr..addr + const { size_of::<N>() as BitSize });
        val.to_le_bytes(buf);

        Ok(())
    }

    /// Reads an address. Fails if addr+N is out of bounds
    pub fn read<N: FromBytes>(&self, addr: BitSize) -> Result<N, MemError> {
        self.validate_addr(const { size_of::<N>() as BitSize }, addr)?;

        let data = self.view(addr..addr + const { size_of::<N>() as BitSize });
        let n = N::from_le_bytes(data);

        Ok(n)
    }

    pub fn view<R: RangeBounds<BitSize>>(&self, r: R) -> &[u8] {
        let start = match r.start_bound() {
            Bound::Included(n) => Bound::Included(*n as usize),
            Bound::Excluded(n) => Bound::Excluded(*n as usize),
            Bound::Unbounded => Bound::Unbounded,
        };

        let end = match r.end_bound() {
            Bound::Included(n) => Bound::Included(*n as usize),
            Bound::Excluded(n) => Bound::Excluded(*n as usize),
            Bound::Unbounded => Bound::Unbounded,
        };

        // SAFETY: alloc is BitSize::MAX big, and range is BitSize
        unsafe { self.data.get_unchecked((start, end)) }
    }

    pub fn view_mut<R: RangeBounds<BitSize>>(&mut self, r: R) -> &mut [u8] {
        let start = match r.start_bound() {
            Bound::Included(n) => Bound::Included(*n as usize),
            Bound::Excluded(n) => Bound::Excluded(*n as usize),
            Bound::Unbounded => Bound::Unbounded,
        };

        let end = match r.end_bound() {
            Bound::Included(n) => Bound::Included(*n as usize),
            Bound::Excluded(n) => Bound::Excluded(*n as usize),
            Bound::Unbounded => Bound::Unbounded,
        };

        // SAFETY: alloc is BitSize::MAX big, and range is BitSize
        unsafe { self.data.get_unchecked_mut((start, end)) }
    }

    /// Validates addr is valid for size and also allocates if needed to make access possible
    fn validate_addr(&self, size: BitSize, addr: BitSize) -> Result<(), MemError> {
        // reads of 0 are always valid regardless of address
        if size == 0 {
            return Ok(());
        }

        // check that size+addr is <= BitSize::MAX
        if addr.checked_add(size).is_none() {
            Err(MemError::InvalidAddr(size, addr))
        } else {
            Ok(())
        }
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        let ptr = self.data.as_ptr().cast::<c_void>().cast_mut();

        let res = unsafe { VirtualFree(ptr, 0, MEM_RELEASE) };

        if let Err(e) = res {
            eprintln!("failed to free mem:\n{e:?}");
        }
    }
}

pub trait ToBytes {
    fn to_ne_bytes(self, buf: &mut [u8]);
    fn to_le_bytes(self, buf: &mut [u8]);
    fn to_be_bytes(self, buf: &mut [u8]);
}

macro_rules! impl_to_bytes {
    ($($ty:ident)+) => ($(
        impl ToBytes for $ty {
            fn to_ne_bytes(self, buf: &mut [u8]) {
                let data = self.to_ne_bytes();
                buf.copy_from_slice(&data);
            }

            fn to_be_bytes(self, buf: &mut [u8]) {
                let data = self.to_be_bytes();
                buf.copy_from_slice(&data);
            }

            fn to_le_bytes(self, buf: &mut [u8]) {
                let data = self.to_le_bytes();
                buf.copy_from_slice(&data);
            }
        }
    )+)
}

impl_to_bytes! { u8 i8 u16 i16 u32 i32 u64 i64 u128 i128 usize isize f32 f64 }

pub trait FromBytes {
    fn from_ne_bytes(buf: &[u8]) -> Self;
    fn from_le_bytes(buf: &[u8]) -> Self;
    fn from_be_bytes(buf: &[u8]) -> Self;
}

macro_rules! impl_from_bytes {
    ($($ty:ident)+) => ($(
        impl FromBytes for $ty {
            fn from_ne_bytes(buf: &[u8]) -> Self {
                let buf: [u8; { size_of::<Self>() }] = buf.try_into().unwrap();
                Self::from_ne_bytes(buf)
            }

            fn from_be_bytes(buf: &[u8]) -> Self {
                let buf: [u8; { size_of::<Self>() }] = buf.try_into().unwrap();
                Self::from_be_bytes(buf)
            }

            fn from_le_bytes(buf: &[u8]) -> Self {
                let buf: [u8; { size_of::<Self>() }] = buf.try_into().unwrap();
                Self::from_le_bytes(buf)
            }
        }
    )+)
}

impl_from_bytes! { u8 i8 u16 i16 u32 i32 u64 i64 u128 i128 usize isize f32 f64 }
