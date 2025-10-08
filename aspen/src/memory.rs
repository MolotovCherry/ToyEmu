use std::{
    ffi::c_void,
    ops::{Index, IndexMut, RangeBounds},
};

use windows::Win32::{
    Foundation::{GetLastError, WIN32_ERROR},
    System::Memory::{
        MEM_COMMIT, MEM_DECOMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc,
        VirtualFree,
    },
};

use crate::BitSize;

#[derive(Debug, Clone, thiserror::Error)]
pub enum MemError {
    #[error("Invalid access: size {0} @ 0x{1:08x}")]
    InvalidAddr(BitSize, BitSize),
    #[error("Alloc failed: {0:?}")]
    Alloc(WIN32_ERROR),
    #[error("Winapi Error: {0}")]
    WinApi(#[from] windows::core::Error),
}

const MEM_SIZE: usize = BitSize::MAX as usize;

pub struct Memory {
    data: *mut [u8; MEM_SIZE],
}

unsafe impl Send for Memory {}

impl<R: RangeBounds<BitSize>> Index<R> for Memory {
    type Output = [u8];

    fn index(&self, index: R) -> &Self::Output {
        let start = index.start_bound().map(|u| *u as usize);
        let end = index.end_bound().map(|u| *u as usize);

        &self.data()[(start, end)]
    }
}

impl<R: RangeBounds<BitSize>> IndexMut<R> for Memory {
    fn index_mut(&mut self, index: R) -> &mut Self::Output {
        let start = index.start_bound().map(|u| *u as usize);
        let end = index.end_bound().map(|u| *u as usize);

        &mut self.data_mut()[(start, end)]
    }
}

impl Memory {
    pub fn new() -> Result<Self, MemError> {
        #[rustfmt::skip]
        let ptr = unsafe {
            VirtualAlloc(
                None,
                MEM_SIZE,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };

        if ptr.is_null() {
            let err = unsafe { GetLastError() };
            return Err(MemError::Alloc(err));
        }

        // SAFETY:
        // alloc is BitSize::MAX big (above)
        // we also already checked for a failed call
        // therefore this cast is valid
        let this = Self {
            data: ptr.cast::<[u8; MEM_SIZE]>(),
        };

        Ok(this)
    }

    /// Write to an address. Fails if addr+val is out of bounds
    pub fn write<N: Copy + ToBytes>(&mut self, addr: BitSize, val: N) -> Result<(), MemError> {
        self.validate_addr(const { size_of::<N>() as BitSize }, addr)?;

        let buf = &mut self[addr..addr + const { size_of::<N>() as BitSize }];
        val.to_be_bytes(buf);

        Ok(())
    }

    /// Reads an address. Fails if addr+N is out of bounds
    pub fn read<N: FromBytes>(&self, addr: BitSize) -> Result<N, MemError> {
        self.validate_addr(const { size_of::<N>() as BitSize }, addr)?;

        let data = &self[addr..addr + const { size_of::<N>() as BitSize }];
        let n = N::from_be_bytes(data);

        Ok(n)
    }

    #[inline]
    pub fn view<R: RangeBounds<BitSize>>(&self, r: R) -> Option<&[u8]> {
        let start = r.start_bound().map(|u| *u as usize);
        let end = r.end_bound().map(|u| *u as usize);

        self.data().get((start, end))
    }

    #[inline]
    pub fn view_mut<R: RangeBounds<BitSize>>(&mut self, r: R) -> Option<&mut [u8]> {
        let start = r.start_bound().map(|u| *u as usize);
        let end = r.end_bound().map(|u| *u as usize);

        self.data_mut().get_mut((start, end))
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

    /// Zeroes memory
    pub fn zeroize(&mut self) -> Result<(), MemError> {
        let ptr = self.data.cast::<c_void>();

        // SAFETY: This call is unique cause &mut self

        // here we will zeroize it by decommitting and recommitting
        // without removing the actual allocation itself
        // when we access memory again after recommit, it will be zeroed

        unsafe {
            VirtualFree(ptr, 0, MEM_DECOMMIT)?;
        }

        #[rustfmt::skip]
        let ptr = unsafe {
            VirtualAlloc(
                Some(ptr),
                MEM_SIZE,
                MEM_COMMIT,
                PAGE_READWRITE
            )
        };

        if ptr.is_null() {
            let err = unsafe { GetLastError() };
            return Err(MemError::Alloc(err));
        }

        Ok(())
    }

    fn data_mut(&mut self) -> &mut [u8; MEM_SIZE] {
        // SAFETY:
        // This ptr is always valid
        // and was created being MEM_SIZE big
        //
        // As for & vs &mut concerns, this is already protected
        // by borrowing from self properly
        unsafe { &mut *self.data }
    }

    fn data(&self) -> &[u8; MEM_SIZE] {
        // SAFETY:
        // This ptr is always valid
        // and was created being MEM_SIZE big
        //
        // As for & vs &mut concerns, this is already protected
        // by borrowing from self properly
        unsafe { &*self.data }
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        let ptr = self.data.cast::<c_void>();

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
