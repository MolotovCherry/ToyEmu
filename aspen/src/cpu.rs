use std::{slice, time::SystemTime};

use bstr::ByteSlice;
use bytemuck::{AnyBitPattern, NoUninit};

use crate::{
    BitSize,
    instruction::{Instruction, InstructionType},
    memory::Memory,
};

#[cfg(feature = "steady-clock")]
use crate::emulator::FREQ;

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum CpuError {
    #[error("{0}")]
    Reg(#[from] RegError),
    #[error("Unsupported instruction: {0:?}")]
    UnsupportedInst(Instruction),
}

#[derive(Default, Copy, Clone)]
pub struct Cpu {
    /// general purpose registers
    pub gp: Registers,
    /// base ptr in memory to display graphics data
    pub gfx: BitSize,
    /// program counter
    pub pc: BitSize,
    /// clock counter
    pub clk: u64,
}

impl Cpu {
    pub fn process(
        &mut self,
        inst: Instruction,
        mem: &mut Memory,
        stop: &mut bool,
        clk: &mut u32,
    ) -> Result<(), CpuError> {
        use InstructionType::*;

        macro_rules! get_imm_or_else {
            ($($t:tt)*) => {{
                if inst.has_imm {
                    inst.imm as _
                } else {
                    $($t)*
                }
            }};
        }

        macro_rules! get_imm_or {
            ($reg:expr) => {{ get_imm_or_else!(self.gp.get_reg($reg)?) }};
        }

        match inst.ty {
            Nop => (),

            Hlt => {
                *stop = true;
                return Ok(());
            }

            Pr => {
                let low = self.gp.get_reg(inst.a)?;
                let high = self.gp.get_reg(inst.b)?;

                if let Some(view) = mem.view(low..high) {
                    let data = view.as_bstr();
                    print!("{data}");
                }
            }

            Epr => {
                let low = self.gp.get_reg(inst.a)?;
                let high = self.gp.get_reg(inst.b)?;

                let data = mem[low..high].as_bstr();
                eprint!("{data}");
            }

            Tme => {
                let time = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);

                let av = (time >> 96) as u32;
                let bv = (time >> 64) as u32;
                let cv = (time >> 32) as u32;
                let dv = time as u32;

                self.gp.set_reg(inst.a, dv)?;
                self.gp.set_reg(inst.b, cv)?;
                self.gp.set_reg(inst.c, bv)?;
                self.gp.set_reg(inst.d, av)?;
            }

            Rdpc => {
                self.gp.set_reg(inst.dst, self.pc)?;
            }

            Kbrd => {
                unimplemented!();
            }

            Setgfx => {
                self.gfx = get_imm_or!(inst.a);
            }

            Draw => {
                unimplemented!();
            }

            Slp => {
                let val = get_imm_or_else! {
                    let val = self.gp.get_reg(inst.a)?;
                    let val2 = self.gp.get_reg(inst.b)?;

                    (val as u64) << 32 | (val2 as u64)
                };

                #[cfg(feature = "steady-clock")]
                if val > 0 {
                    // add cycles consistent with frequency
                    let fre = const { FREQ.as_micros() as u64 };
                    // adjust the clock frequency scaled by our wait time
                    // waits in multiples of FREQ
                    *clk = val.max(fre).div_ceil(fre) as u32;
                }
            }

            Rdclk => {
                let high = (self.clk >> 32) as BitSize;
                let low = (self.clk & 0xffffffff) as BitSize;

                self.gp.set_reg(inst.a, low)?;
                self.gp.set_reg(inst.b, high)?;
            }

            #[rustfmt::skip]
            //
            // Memory
            //

            Ld => {}

            Ldw => {}

            Ldb => {}

            Pld => {}

            Pldw => {}

            Pldb => {}

            Str => {}

            Strw => {}

            Strb => {}

            Pstr => {}

            Pstrw => {}

            Pstrb => {}

            #[rustfmt::skip]
            //
            // MATH
            //

            Nand => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, !(a & b))?;
            }

            Or => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, a | b)?;
            }

            And => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, a & b)?;
            }

            Nor => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, !(a | b))?;
            }

            Add => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, a.wrapping_add(b))?;
            }

            Sub => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, a.wrapping_sub(b))?;
            }

            Xor => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, a ^ b)?;
            }

            Lsl => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, a << b)?;
            }

            Lsr => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, a >> b)?;
            }

            Mul => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, a.wrapping_mul(b))?;
            }

            Imul => {
                let a = self.gp.get_reg(inst.a)? as i32;
                let b = get_imm_or!(inst.b) as i32;

                self.gp.set_reg(inst.dst, a.wrapping_mul(b) as u32)?;
            }

            Div => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                let val = if a != 0 { a.wrapping_div(b) } else { 0 };
                self.gp.set_reg(inst.dst, val)?;
            }

            Idiv => {
                let a = self.gp.get_reg(inst.a)? as i32;
                let b = get_imm_or!(inst.b) as i32;

                let val = if a != 0 { a.wrapping_div(b) } else { 0 };
                self.gp.set_reg(inst.dst, val as u32)?;
            }

            Rem => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, a % b)?;
            }

            Irem => {
                let a = self.gp.get_reg(inst.a)? as i32;
                let b = get_imm_or!(inst.b) as i32;

                self.gp.set_reg(inst.dst, (a % b) as u32)?;
            }

            Mov => {
                let a = get_imm_or!(inst.b);
                self.gp.set_reg(inst.dst, a)?;
            }

            Inc => {
                let a = self.gp.get_reg(inst.a)?.wrapping_add(1);
                self.gp.set_reg(inst.dst, a)?;
            }

            Dec => {
                let a = self.gp.get_reg(inst.a)?.wrapping_sub(1);
                self.gp.set_reg(inst.dst, a)?;
            }

            Se => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, (a == b) as _)?;
            }

            Sne => {
                let a = self.gp.get_reg(inst.a)?;
                let b = get_imm_or!(inst.b);

                self.gp.set_reg(inst.dst, (a != b) as _)?;
            }

            Sl => {
                let a = self.gp.get_reg(inst.a)? as i32;
                let b = get_imm_or!(inst.b) as i32;

                self.gp.set_reg(inst.dst, (a < b) as _)?;
            }

            Sle => {
                let a = self.gp.get_reg(inst.a)? as i32;
                let b = get_imm_or!(inst.b) as i32;

                self.gp.set_reg(inst.dst, (a <= b) as _)?;
            }

            Sg => {
                let a = self.gp.get_reg(inst.a)? as i32;
                let b = get_imm_or!(inst.b) as i32;

                self.gp.set_reg(inst.dst, (a > b) as _)?;
            }

            Sge => {
                let a = self.gp.get_reg(inst.a)? as i32;
                let b = get_imm_or!(inst.b) as i32;

                self.gp.set_reg(inst.dst, (a >= b) as _)?;
            }

            Asr => {
                let a = self.gp.get_reg(inst.a)? as i32;
                let b = get_imm_or!(inst.b) as i32;

                self.gp.set_reg(inst.dst, (a >> b) as u32)?;
            }

            #[rustfmt::skip]
            //
            // CONDITIONALS
            //

            Jmp => {
                let dst = get_imm_or!(inst.dst);
                self.pc = dst;
                return Ok(());
            }

            Je => {
                let dst = get_imm_or!(inst.dst);

                let a = self.gp.get_reg(inst.a)?;
                let b = self.gp.get_reg(inst.b)?;

                if a == b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            Jne => {
                let dst = get_imm_or!(inst.dst);

                let a = self.gp.get_reg(inst.a)?;
                let b = self.gp.get_reg(inst.b)?;

                if a != b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            Jl => {
                let dst = get_imm_or!(inst.dst);

                let a = self.gp.get_reg(inst.a)? as i32;
                let b = self.gp.get_reg(inst.b)? as i32;

                if a < b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            Jge => {
                let dst = get_imm_or!(inst.dst);

                let a = self.gp.get_reg(inst.a)? as i32;
                let b = self.gp.get_reg(inst.b)? as i32;

                if a >= b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            Jle => {
                let dst = get_imm_or!(inst.dst);

                let a = self.gp.get_reg(inst.a)? as i32;
                let b = self.gp.get_reg(inst.b)? as i32;

                if a <= b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            Jg => {
                let dst = get_imm_or!(inst.dst);

                let a = self.gp.get_reg(inst.a)? as i32;
                let b = self.gp.get_reg(inst.b)? as i32;

                if a > b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            Jb => {
                let dst = get_imm_or!(inst.dst);

                let a = self.gp.get_reg(inst.a)?;
                let b = self.gp.get_reg(inst.b)?;

                if a < b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            Jae => {
                let dst = get_imm_or!(inst.dst);

                let a = self.gp.get_reg(inst.a)?;
                let b = self.gp.get_reg(inst.b)?;

                if a >= b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            Jbe => {
                let dst = get_imm_or!(inst.dst);

                let a = self.gp.get_reg(inst.a)?;
                let b = self.gp.get_reg(inst.b)?;

                if a <= b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            Ja => {
                let dst = get_imm_or!(inst.dst);

                let a = self.gp.get_reg(inst.a)?;
                let b = self.gp.get_reg(inst.b)?;

                if a > b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            #[rustfmt::skip]
            //
            // STACK
            //

            Push => {
                let a = self.gp.get_reg(inst.a)?;
                let old_sp = self.gp.sp;

                self.gp.sp = self.gp.sp.wrapping_sub(size_of::<BitSize>() as _);

                let slice = if old_sp < self.gp.sp {
                    &mut mem[self.gp.sp..]
                } else {
                    &mut mem[self.gp.sp..old_sp]
                };

                slice.copy_from_slice(&a.to_le_bytes());

                *clk = 2;
            }

            Pop => {
                let bytes = &mem[self.gp.sp..self.gp.sp + size_of::<BitSize>() as BitSize];
                let data = BitSize::from_le_bytes(bytes.try_into().unwrap());
                self.gp.sp = self.gp.sp.wrapping_add(size_of::<BitSize>() as _);
                self.gp.set_reg(inst.dst, data)?;

                *clk = 2;
            }

            Call => {
                // push old ra to stack
                let old_sp = self.gp.sp;
                self.gp.sp = self.gp.sp.wrapping_sub(size_of::<BitSize>() as _);
                mem[self.gp.sp..old_sp].copy_from_slice(&self.gp.ra.to_le_bytes());

                let jmp = get_imm_or!(inst.a);

                // return back to current pc
                self.gp.ra = self.pc;
                // make sure to set to next instruction
                self.gp.ra += if inst.has_imm { 8 } else { 4 };

                // set pc to new loc
                self.pc = jmp;

                *clk = 3;

                return Ok(());
            }

            Ret => {
                // jmp to return addr
                self.pc = self.gp.ra;

                // pop old ra off stack and set it
                let bytes = &mem[self.gp.sp..self.gp.sp + size_of::<BitSize>() as BitSize];
                let ra = BitSize::from_le_bytes(bytes.try_into().unwrap());
                self.gp.sp = self.gp.sp.wrapping_add(size_of::<BitSize>() as _);
                self.gp.ra = ra;

                *clk = 2;

                return Ok(());
            }
        }

        self.pc += if inst.has_imm { 8 } else { 4 };

        Ok(())
    }

    /// zero all registers
    #[allow(unused)]
    pub fn zeroize(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("Invalid register: 0x{0:02x}")]
pub struct RegError(u8);

/// Accessible CPU registers
///
/// \[r\] - caller saved
/// \[e\] - callee saved
#[repr(C)]
#[derive(Copy, Clone, NoUninit, AnyBitPattern)]
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

impl Default for Registers {
    fn default() -> Self {
        Self {
            zr: Default::default(),
            ra: Default::default(),
            sp: BitSize::MAX,
            gp: Default::default(),
            tp: Default::default(),
            t0: Default::default(),
            t1: Default::default(),
            t2: Default::default(),
            t3: Default::default(),
            t4: Default::default(),
            t5: Default::default(),
            t6: Default::default(),
            s0: Default::default(),
            s1: Default::default(),
            s2: Default::default(),
            s3: Default::default(),
            s4: Default::default(),
            s5: Default::default(),
            s6: Default::default(),
            s7: Default::default(),
            s8: Default::default(),
            s9: Default::default(),
            s10: Default::default(),
            s11: Default::default(),
            a0: Default::default(),
            a1: Default::default(),
            a2: Default::default(),
            a3: Default::default(),
            a4: Default::default(),
            a5: Default::default(),
            a6: Default::default(),
            a7: Default::default(),
        }
    }
}

impl Registers {
    #[inline]
    fn array(&self) -> &[BitSize] {
        const {
            assert!(
                size_of::<Self>().is_multiple_of(size_of::<BitSize>()),
                "Registers size does not fit evenly"
            );
        }

        let slice = slice::from_ref(self);
        bytemuck::must_cast_slice(slice)
    }

    #[inline]
    fn array_mut(&mut self) -> &mut [BitSize] {
        const {
            assert!(
                size_of::<Self>().is_multiple_of(size_of::<BitSize>()),
                "Registers size does not fit evenly"
            );
        }

        let slice = slice::from_mut(self);
        bytemuck::must_cast_slice_mut(slice)
    }

    /// Set register based on index
    #[inline]
    pub fn set_reg(&mut self, reg: u8, val: BitSize) -> Result<(), RegError> {
        // zr is a noop
        if reg == 0 {
            return Ok(());
        }

        let elem = self
            .array_mut()
            .get_mut(reg as usize)
            .ok_or(RegError(reg))?;

        *elem = val;

        Ok(())
    }

    /// Read register based on index
    #[inline]
    pub fn get_reg(&self, reg: u8) -> Result<BitSize, RegError> {
        let elem = *self.array().get(reg as usize).ok_or(RegError(reg))?;

        Ok(elem)
    }

    pub fn mnemonic(reg: u8) -> &'static str {
        match reg {
            0x00 => "zr",
            0x01 => "ra",
            0x02 => "sp",
            0x03 => "gp",
            0x04 => "tp",
            0x05 => "t0",
            0x06 => "t1",
            0x07 => "t2",
            0x08 => "t3",
            0x09 => "t4",
            0x0a => "t5",
            0x0b => "t6",
            0x0c => "s0",
            0x0d => "s1",
            0x0e => "s2",
            0x0f => "s3",
            0x10 => "s4",
            0x11 => "s5",
            0x12 => "s6",
            0x13 => "s7",
            0x14 => "s8",
            0x15 => "s9",
            0x16 => "s10",
            0x17 => "s11",
            0x18 => "a0",
            0x19 => "a1",
            0x1a => "a2",
            0x1b => "a3",
            0x1c => "a4",
            0x1d => "a5",
            0x1e => "a6",
            0x1f => "a7",

            _ => "<un>",
        }
    }
}
