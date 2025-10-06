use std::slice;

use bytemuck::{AnyBitPattern, NoUninit};

use crate::BitSize;

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("Invalid register: 0x{0:02x}")]
pub struct RegError(u8);

#[derive(Default, Copy, Clone)]
pub struct Cpu {
    pub gp: Registers,
    /// base ptr in memory to display graphics data
    pub gfx: BitSize,
    /// program counter
    pub pc: BitSize,
}

/// Accessible CPU registers
///
/// \[r\] - caller saved
/// \[e\] - callee saved
#[repr(C)]
#[derive(Copy, Clone, Default, NoUninit, AnyBitPattern)]
pub struct Registers {
    /// zero register
    pub zr: BitSize,
    /// \[r\] return address
    pub ra: BitSize,
    /// stack pointer
    pub sp: BitSize,
    /// global pointer
    pub gp: BitSize,
    /// thread pointer
    pub tp: BitSize,
    /// \[r\] temporary 0
    pub t0: BitSize,
    /// \[r\] temporary 1
    pub t1: BitSize,
    /// \[r\] temporary 2
    pub t2: BitSize,
    /// \[r\] temporary 3
    pub t3: BitSize,
    /// \[r\] temporary 4
    pub t4: BitSize,
    /// \[r\] temporary 5
    pub t5: BitSize,
    /// \[r\] temporary 6
    pub t6: BitSize,
    /// \[e\] saved 0 / frame pointer
    pub s0: BitSize,
    /// \[e\] saved 1
    pub s1: BitSize,
    /// \[e\] saved 2
    pub s2: BitSize,
    /// \[e\] saved 3
    pub s3: BitSize,
    /// \[e\] saved 4
    pub s4: BitSize,
    /// \[e\] saved 5
    pub s5: BitSize,
    /// \[e\] saved 6
    pub s6: BitSize,
    /// \[e\] saved 7
    pub s7: BitSize,
    /// \[e\] saved 8
    pub s8: BitSize,
    /// \[e\] saved 9
    pub s9: BitSize,
    /// \[e\] saved 10
    pub s10: BitSize,
    /// \[e\] saved 11
    pub s11: BitSize,
    /// \[r\] function argument 0 / return value 0
    pub a0: BitSize,
    /// \[r\] function argument 1 / return value 1
    pub a1: BitSize,
    /// \[r\] function argument 2
    pub a2: BitSize,
    /// \[r\] function argument 3
    pub a3: BitSize,
    /// \[r\] function argument 4
    pub a4: BitSize,
    /// \[r\] function argument 5
    pub a5: BitSize,
    /// \[r\] function argument 6
    pub a6: BitSize,
    /// \[r\] function argument 7
    pub a7: BitSize,
}

impl Registers {
    /// Access registers as an array for easy
    pub fn as_array_mut(&mut self) -> &mut [BitSize] {
        const {
            assert!(
                size_of::<Self>().is_multiple_of(size_of::<BitSize>()),
                "Registers size does not fit evenly"
            );
        }

        let slice = slice::from_mut(self);
        bytemuck::must_cast_slice_mut(slice)
    }

    /// Access registers as an array for easy
    pub fn as_array(&self) -> &[BitSize] {
        const {
            assert!(
                size_of::<Self>().is_multiple_of(size_of::<BitSize>()),
                "Registers size does not fit evenly"
            );
        }

        let slice = slice::from_ref(self);
        bytemuck::must_cast_slice(slice)
    }

    /// Set register based on index
    pub fn set(&mut self, reg: u8, val: BitSize) -> Result<(), RegError> {
        let error = RegError(reg);

        let idx = (reg as usize)
            .checked_mul(size_of::<BitSize>())
            .ok_or(error)?;

        let elem = self.as_array_mut().get_mut(idx).ok_or(error)?;
        *elem = val;

        Ok(())
    }

    /// Read register based on index
    pub fn get(&self, reg: u8) -> Result<BitSize, RegError> {
        let error = RegError(reg);

        let idx = (reg as usize)
            .checked_mul(size_of::<BitSize>())
            .ok_or(error)?;

        let elem = self.as_array().get(idx).ok_or(error)?;
        Ok(*elem)
    }
}
