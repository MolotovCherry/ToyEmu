use std::{
    ffi::c_void,
    marker::PhantomData,
    ops::{
        Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo,
        RangeToInclusive,
    },
};

use enumflags2::{BitFlags, bitflags};

use crate::BitSize;

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum MemError {
    #[error("Invalid access: size {0} @ 0x{1:08x}")]
    InvalidAddr(BitSize, BitSize),
    #[cfg(windows)]
    #[error("Alloc failed: {0:?}")]
    Alloc(windows::Win32::Foundation::WIN32_ERROR),
    #[cfg(windows)]
    #[error("Winapi Error: {0}")]
    WinApi(#[from] windows::core::Error),
    #[error("Page fault: {0} access denied")]
    PageFault(BitFlags<Prot>),
    #[error("Failed to change Prot")]
    Overflow,
    #[cfg(unix)]
    #[error("I/O Error: {0}")]
    Io(std::sync::Arc<std::io::Error>),
}

const MEM_SIZE: usize = BitSize::MAX as usize + 1;
pub const PAGE_SIZE: usize = {
    let size = 4096;
    assert!(MEM_SIZE.is_multiple_of(size));
    size
};

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

#[derive(Debug)]
pub struct Memory {
    data: *mut [u8; MEM_SIZE],
    pages: Vec<BitFlags<Prot>>,
    phantom: PhantomData<Box<[u8; MEM_SIZE]>>,
}

unsafe impl Send for Memory {}

impl Memory {
    #[cfg(windows)]
    pub fn new() -> Result<Self, MemError> {
        use windows::Win32::{
            Foundation::GetLastError,
            System::Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc},
        };

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

        let rw = Prot::Read | Prot::Write;
        let pages = vec![rw; MEM_SIZE / PAGE_SIZE];

        // SAFETY:
        // alloc is BitSize::MAX big (above)
        // we also already checked for a failed call
        // therefore this cast is valid
        let this = Self {
            data: ptr.cast::<[u8; MEM_SIZE]>(),
            pages,
            phantom: PhantomData,
        };

        Ok(this)
    }

    #[cfg(unix)]
    pub fn new() -> Result<Self, MemError> {
        use core::ptr::addr_eq;
        use core::{mem::transmute, ptr::null_mut};
        use libc::{MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_READ, PROT_WRITE, mmap};
        use std::os::fd::BorrowedFd;

        const INVALID_FD: i32 = -1;

        let ptr = unsafe {
            mmap(
                null_mut(),
                MEM_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                INVALID_FD,
                0,
            )
        };

        if addr_eq(ptr, MAP_FAILED) {
            let err = io::Error::last_os_error();

            return Err(MemError::Io(Arc::new(err)));
        }

        let rw = Prot::Read | Prot::Write;
        let pages = vec![rw; MEM_SIZE / PAGE_SIZE];

        // SAFETY:
        // alloc is BitSize::MAX big (above)
        // we also already checked for a failed call
        // therefore this cast is valid
        let this = Self {
            data: ptr.cast::<[u8; MEM_SIZE]>(),
            pages,
            phantom: PhantomData,
        };

        Ok(this)
    }

    /// Write to an address. Fails if addr+val is out of bounds or if page is not writeable
    pub fn write<N: Copy + ToBytes>(&mut self, addr: BitSize, val: N) -> Result<(), MemError> {
        self.validate_addr(
            const { size_of::<N>() as BitSize },
            addr,
            Prot::Write.into(),
        )?;

        let buf = &mut self.data_mut()
            [addr as usize..(addr + const { size_of::<N>() as BitSize }) as usize];
        val.to_be_bytes(buf);

        Ok(())
    }

    /// Reads an address. Fails if addr+N is out of bounds or if page is not readable
    pub fn read<N: FromBytes>(&self, addr: BitSize) -> Result<N, MemError> {
        self.validate_addr(const { size_of::<N>() as BitSize }, addr, Prot::Read.into())?;

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

    /// Validates addr is valid for size and prot
    fn validate_addr(
        &self,
        size: BitSize,
        addr: BitSize,
        prot: BitFlags<Prot>,
    ) -> Result<(), MemError> {
        // reads of 0 are always valid regardless of address
        if size == 0 {
            return Ok(());
        }

        // check that size+addr is <= BitSize::MAX
        if addr.checked_add(size).is_none() {
            return Err(MemError::InvalidAddr(size, addr));
        }

        self.check_prot(addr..addr, prot)?;

        Ok(())
    }

    fn page_idx_of(addr: BitSize) -> usize {
        (addr / PAGE_SIZE as u32) as usize
    }

    pub fn check_prot<R: IntoRange>(&self, addr: R, req: BitFlags<Prot>) -> Result<(), MemError> {
        let (start, end) = addr.into_range();

        for addr in (start..=end).step_by(PAGE_SIZE) {
            let idx = Self::page_idx_of(addr);
            let record = self.pages[idx];
            if !record.contains(req) {
                let i = !record & req;
                return Err(MemError::PageFault(i));
            }
        }

        Ok(())
    }

    pub fn change_prot<R: IntoRange>(
        &mut self,
        addr: R,
        req: BitFlags<Prot>,
    ) -> Result<(), MemError> {
        let (start, end) = addr.into_range();

        for addr in (start..=end).step_by(PAGE_SIZE) {
            let idx = Self::page_idx_of(addr);
            self.pages[idx] = req;
        }

        Ok(())
    }

    /// Zeroes memory
    #[allow(unused)]
    #[cfg(windows)]
    pub fn zeroize(&mut self) -> Result<(), MemError> {
        use windows::Win32::{
            Foundation::{GetLastError, WIN32_ERROR},
            System::Memory::{
                MEM_COMMIT, MEM_DECOMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc,
                VirtualFree,
            },
        };

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

    #[allow(unused)]
    #[cfg(unix)]
    pub fn zeroize(&mut self) -> Result<(), MemError> {
        let ptr = self.data.cast::<c_void>();

        // SAFETY:
        // `DONT_NEED` has the effects of resetting the backing memory to zeroes immediately
        // we can't use `MADV_FREE` on linux because it's a delayed operation which means
        // the memory is effectively "uninit" and/or "aliased" since it could change at
        // any random point in time
        //
        // this also lets the operating system reclaim the pages we wrote to
        unsafe { libc::madvise(ptr, MEM_SIZE, libc::MADV_DONTNEED) };

        Ok(())
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        // SAFETY:
        // This ptr is always valid
        // and was created being MEM_SIZE big
        //
        // As for & vs &mut concerns, this is already protected
        // by borrowing from self properly
        unsafe { &mut *self.data }
    }

    pub fn data(&self) -> &[u8] {
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
    #[cfg(windows)]
    fn drop(&mut self) {
        use windows::Win32::System::Memory::{MEM_RELEASE, VirtualFree};

        let ptr = self.data.cast::<c_void>();

        let res = unsafe { VirtualFree(ptr, 0, MEM_RELEASE) };

        if let Err(e) = res {
            eprintln!("failed to free mem:\n{e:?}");
        }
    }

    #[cfg(unix)]
    fn drop(&mut self) {
        let ptr = self.data.cast::<c_void>();

        let res = unsafe { libc::munmap(ptr, MEM_SIZE) };

        if res == -1 {
            eprintln!("failed to free mem:\n{:?}", io::Error::last_os_error());
        }
    }
}

impl Index<BitSize> for Memory {
    type Output = u8;

    fn index(&self, index: BitSize) -> &Self::Output {
        &self.data()[index as usize]
    }
}

impl Index<Range<BitSize>> for Memory {
    type Output = [u8];

    fn index(&self, index: Range<BitSize>) -> &Self::Output {
        let index = Range {
            start: index.start as _,
            end: index.end as _,
        };

        &self.data()[index]
    }
}

impl Index<RangeFrom<BitSize>> for Memory {
    type Output = [u8];

    fn index(&self, index: RangeFrom<BitSize>) -> &Self::Output {
        let index = RangeFrom {
            start: index.start as _,
        };

        &self.data()[index]
    }
}

impl Index<RangeFull> for Memory {
    type Output = [u8];

    fn index(&self, _: RangeFull) -> &Self::Output {
        self.data()
    }
}

impl Index<RangeInclusive<BitSize>> for Memory {
    type Output = [u8];

    fn index(&self, index: RangeInclusive<BitSize>) -> &Self::Output {
        let index = RangeInclusive::new(*index.start() as usize, *index.end() as usize);

        &self.data()[index]
    }
}

impl Index<RangeTo<BitSize>> for Memory {
    type Output = [u8];

    fn index(&self, index: RangeTo<BitSize>) -> &Self::Output {
        let index = RangeTo {
            end: index.end as _,
        };

        &self.data()[index]
    }
}

impl Index<RangeToInclusive<BitSize>> for Memory {
    type Output = [u8];

    fn index(&self, index: RangeToInclusive<BitSize>) -> &Self::Output {
        let index = RangeToInclusive {
            end: index.end as _,
        };

        &self.data()[index]
    }
}

impl IndexMut<BitSize> for Memory {
    fn index_mut(&mut self, index: BitSize) -> &mut Self::Output {
        &mut self.data_mut()[index as usize]
    }
}

impl IndexMut<Range<BitSize>> for Memory {
    fn index_mut(&mut self, index: Range<BitSize>) -> &mut Self::Output {
        let index = Range {
            start: index.start as _,
            end: index.end as _,
        };

        &mut self.data_mut()[index]
    }
}

impl IndexMut<RangeFrom<BitSize>> for Memory {
    fn index_mut(&mut self, index: RangeFrom<BitSize>) -> &mut Self::Output {
        let index = RangeFrom {
            start: index.start as _,
        };

        &mut self.data_mut()[index]
    }
}

impl IndexMut<RangeFull> for Memory {
    fn index_mut(&mut self, _: RangeFull) -> &mut Self::Output {
        self.data_mut()
    }
}

impl IndexMut<RangeInclusive<BitSize>> for Memory {
    fn index_mut(&mut self, index: RangeInclusive<BitSize>) -> &mut Self::Output {
        let index = RangeInclusive::new(*index.start() as usize, *index.end() as usize);

        &mut self.data_mut()[index]
    }
}

impl IndexMut<RangeTo<BitSize>> for Memory {
    fn index_mut(&mut self, index: RangeTo<BitSize>) -> &mut Self::Output {
        let index = RangeTo {
            end: index.end as _,
        };

        &mut self.data_mut()[index]
    }
}

impl IndexMut<RangeToInclusive<BitSize>> for Memory {
    fn index_mut(&mut self, index: RangeToInclusive<BitSize>) -> &mut Self::Output {
        let index = RangeToInclusive {
            end: index.end as _,
        };

        &mut self.data_mut()[index]
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

/// Allows both all types of range + single num as input
pub trait IntoRange {
    /// note: both sides are inclusive
    fn into_range(self) -> (u32, u32);
}

impl IntoRange for u32 {
    fn into_range(self) -> (u32, u32) {
        (self, self)
    }
}

impl IntoRange for Range<BitSize> {
    fn into_range(self) -> (u32, u32) {
        (self.start, self.end.saturating_sub(1))
    }
}

impl IntoRange for RangeFrom<BitSize> {
    fn into_range(self) -> (u32, u32) {
        (self.start, BitSize::MAX)
    }
}

impl IntoRange for RangeFull {
    fn into_range(self) -> (u32, u32) {
        (BitSize::MIN, BitSize::MAX)
    }
}

impl IntoRange for RangeInclusive<BitSize> {
    fn into_range(self) -> (u32, u32) {
        (*self.start(), *self.end())
    }
}

impl IntoRange for RangeTo<BitSize> {
    fn into_range(self) -> (u32, u32) {
        (BitSize::MIN, self.end.saturating_sub(1))
    }
}

impl IntoRange for RangeToInclusive<BitSize> {
    fn into_range(self) -> (u32, u32) {
        (BitSize::MIN, self.end)
    }
}
