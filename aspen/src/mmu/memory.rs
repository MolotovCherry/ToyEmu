use std::{
    arch::asm,
    ffi::c_void,
    marker::PhantomData,
    slice,
    sync::atomic::{AtomicU8, Ordering},
};

#[cfg(windows)]
use crate::mmu::MemError;
use crate::{
    BitSize,
    mmu::{MEM_SIZE, address_range::AddressRange},
};

#[doc(hidden)]
#[derive(Debug)]
pub struct Memory {
    data: *mut [AtomicU8; MEM_SIZE],
    phantom: PhantomData<Box<[AtomicU8; MEM_SIZE]>>,
}

// We exclusively own and manage the memory
unsafe impl Send for Memory {}
// Reading and writing safety is guaranteed by the caller, the methods are unsafe
unsafe impl Sync for Memory {}

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

        // SAFETY:
        // alloc is BitSize::MAX big (above)
        // we also already checked for a failed call
        // therefore this cast is valid
        let this = Self {
            data: ptr.cast::<[_; MEM_SIZE]>(),
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
            let err = std::io::Error::last_os_error();

            return Err(MemError::Io(Arc::new(err)));
        }

        // SAFETY:
        // alloc is BitSize::MAX big (above)
        // we also already checked for a failed call
        // therefore this cast is valid
        let this = Self {
            data: ptr.cast::<[_; MEM_SIZE]>(),
            phantom: PhantomData,
        };

        Ok(this)
    }

    fn slice(&self, addr: impl Into<AddressRange>) -> &[AtomicU8] {
        let addr = addr.into();

        // SAFETY: addr is limited to BitSize, alloc is BitSize::MAX big
        // so it's within the alloc. Also, BitSize < isize::MAX (see assert above)
        const { assert!((BitSize::MAX as usize) <= isize::MAX as usize) }
        let ptr = unsafe { self.data.cast::<AtomicU8>().add(addr.start as usize) };

        // do not wraparound since that would pointlessly cause a massive slice
        let len = addr.end.saturating_sub(addr.start.saturating_sub(1));

        unsafe { slice::from_raw_parts(ptr, len as usize) }
    }

    /// Write to an address.
    pub fn write<N: Copy + ToBytes>(&self, addr: BitSize, val: N) -> Result<(), MemError> {
        let mut buf = N::Buf::default();
        val.to_le_bytes(&mut buf);

        // 0 is inclusive and 0+size-1 is also inclusive, so sub 1 is important here for the overflow check
        let end = addr
            .checked_add((size_of::<N>() as BitSize).saturating_sub(1))
            .ok_or(MemError::Overflow)?;

        let data = self.slice(addr..=end);

        for (a, v) in data.iter().zip(buf) {
            a.store(v, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Read an address.
    pub fn read<N: FromBytes>(&self, addr: BitSize) -> Result<N, MemError> {
        let end = addr
            .checked_add((size_of::<N>() as BitSize).saturating_sub(1))
            .ok_or(MemError::Overflow)?;

        let data = self.slice(addr..=end);

        let mut buf = N::Buf::default();
        N::copy_from_atomic_slice(&mut buf, data);

        let n = N::from_le_bytes(&buf);
        Ok(n)
    }

    /// Starting at addr, copies buf.len bytes into buf
    pub fn memcpy(&self, addr: BitSize, buf: &mut [u8]) -> Result<(), MemError> {
        let end = addr
            .checked_add(buf.len().saturating_sub(1) as _)
            .ok_or(MemError::Overflow)?;

        let data = self.slice(addr..=end);

        if cfg!(target_arch = "x86_64") {
            unsafe {
                asm!(
                    "rep movsb",
                    inout("rcx") buf.len() =>  _,
                    inout("rsi") data.as_ptr() =>  _,
                    inout("rdi") buf.as_mut_ptr() =>  _,
                    options(nostack, preserves_flags)
                )
            }
        } else {
            for (a, b) in data.iter().zip(buf) {
                *b = a.load(Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Write to mem using memcpy
    pub fn memwrite(&self, addr: BitSize, buf: &[u8]) -> Result<(), MemError> {
        let end = addr
            .checked_add(buf.len().saturating_sub(1) as _)
            .ok_or(MemError::Overflow)?;

        let data = self.slice(addr..=end);

        for (a, b) in data.iter().zip(buf) {
            a.store(*b, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Access raw mem
    ///
    /// # Safety
    /// No read or writes of any kind are allowed while this slice is alive
    pub unsafe fn mem(&self) -> &[u8; MEM_SIZE] {
        unsafe { &*self.data.cast() }
    }

    /// Access raw mutable mem
    ///
    /// # Safety
    /// No read or writes of any kind are allowed while this slice is alive
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn mem_mut(&self) -> &mut [u8; MEM_SIZE] {
        unsafe { &mut *self.data.cast() }
    }

    /// Zeroes memory
    ///
    /// # Safety
    ///
    /// No other reads/writes can happen, or views can exist, while this is executing
    #[cfg(windows)]
    pub unsafe fn zeroize(&self) -> Result<(), MemError> {
        use windows::Win32::{
            Foundation::GetLastError,
            System::Memory::{MEM_COMMIT, MEM_DECOMMIT, PAGE_READWRITE, VirtualAlloc, VirtualFree},
        };

        let ptr = self.data.cast::<c_void>();

        // SAFETY: Caller guarantees no other reads/write happen or views exist

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

    /// Zeroes memory
    ///
    /// # Safety
    ///
    /// No other reads/writes must be happening, or views can exist, until this is finished
    #[cfg(unix)]
    pub unsafe fn zeroize(&mut self) -> Result<(), MemError> {
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
            eprintln!("failed to free mem:\n{:?}", std::io::Error::last_os_error());
        }
    }
}

pub trait ToBytes {
    type Buf: Default + IntoIterator<Item = u8>;

    fn to_ne_bytes(self, buf: &mut Self::Buf);
    fn to_le_bytes(self, buf: &mut Self::Buf);
    fn to_be_bytes(self, buf: &mut Self::Buf);
}

macro_rules! impl_to_bytes {
    ($($ty:ident)+) => ($(
        impl ToBytes for $ty {
            type Buf = [u8; size_of::<Self>()];

            fn to_ne_bytes(self, buf: &mut Self::Buf) {
                let data = self.to_ne_bytes();
                buf.copy_from_slice(&data);
            }

            fn to_be_bytes(self, buf: &mut Self::Buf) {
                let data = self.to_be_bytes();
                buf.copy_from_slice(&data);
            }

            fn to_le_bytes(self, buf: &mut Self::Buf) {
                let data = self.to_le_bytes();
                buf.copy_from_slice(&data);
            }
        }
    )+)
}

impl_to_bytes! { u8 i8 u16 i16 u32 i32 u64 i64 u128 i128 usize isize f32 f64 }

pub trait FromBytes {
    type Buf: Default;

    fn copy_from_atomic_slice(buf: &mut Self::Buf, data: &[AtomicU8]);

    fn from_ne_bytes(buf: &Self::Buf) -> Self;
    fn from_le_bytes(buf: &Self::Buf) -> Self;
    fn from_be_bytes(buf: &Self::Buf) -> Self;
}

macro_rules! impl_from_bytes {
    ($($ty:ident)+) => ($(
        impl FromBytes for $ty {
            type Buf = [u8; size_of::<Self>()];

            fn copy_from_atomic_slice(buf: &mut Self::Buf, data: &[AtomicU8]) {
                for (buf_byte, byte) in buf.iter_mut().zip(data) {
                    *buf_byte = byte.load(Ordering::Relaxed);
                }
            }

            fn from_ne_bytes(buf: &Self::Buf) -> Self {
                Self::from_ne_bytes(*buf)
            }

            fn from_be_bytes(buf: &Self::Buf) -> Self {
                Self::from_be_bytes(*buf)
            }

            fn from_le_bytes(buf: &Self::Buf) -> Self {
                Self::from_le_bytes(*buf)
            }
        }
    )+)
}

impl_from_bytes! { u8 i8 u16 i16 u32 i32 u64 i64 u128 i128 usize isize f32 f64 }
