use std::{
    ops::{Deref, DerefMut},
    process, slice, thread,
    time::{Duration, SystemTime},
};

use bitflags::bitflags;
use bstr::ByteSlice;
use bytemuck::{AnyBitPattern, NoUninit};
use log::trace;

use crate::{BitSize, instruction::Instruction, memory::Memory};

bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct Flags: BitSize {
        /// Zero Flag
        const Z = 0b00000001;
        /// Carry Flag
        const C = 0b00000010;
        /// Sign Flag
        const S = 0b00000100;
        /// Overflow Flag
        const O = 0b00001000;
    }
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum CpuError {
    #[error("{0}")]
    Reg(#[from] RegError),
}

#[derive(Default, Copy, Clone)]
pub struct Cpu {
    /// general purpose registers
    pub gp: Registers,
    /// base ptr in memory to display graphics data
    pub gfx: BitSize,
    /// program counter
    pub pc: BitSize,
}

impl Cpu {
    pub fn process(&mut self, inst: Instruction, mem: &mut Memory) -> Result<(), CpuError> {
        match (inst.mode, inst.dst, inst.op_code, inst.a, inst.b, inst.imm) {
            // nop
            (0, _, 0x0, _, _, None) => trace!("nop"),

            // hlt
            (0, _, 0x1, a, _, None) => {
                trace!("hlt {}", Self::mnemonic(a));
                let val = self.gp.get_reg(a)?;
                process::exit(val as _);
            }

            // pr {reg}, {reg}
            (0, _, 0x2, a, b, _) => {
                trace!("pr {}, {}", Self::mnemonic(a), Self::mnemonic(b));

                let low = self.gp.get_reg(a)?;
                let high = self.gp.get_reg(b)?;

                let data = mem[low..high].as_bstr();
                print!("{data}");
            }

            // epr {reg}, {reg}
            (0, _, 0x3, a, b, _) => {
                trace!("epr {}, {}", Self::mnemonic(a), Self::mnemonic(b));

                let low = self.gp.get_reg(a)?;
                let high = self.gp.get_reg(b)?;

                let data = mem[low..high].as_bstr();
                eprint!("{data}");
            }

            // time {reg}, {reg}, {reg}, {reg}
            (0, _, 0x4, a, b, Some(i)) => {
                let [c, d, _, _] = i.to_be_bytes();

                trace!(
                    "time {}, {}, {}, {}",
                    Self::mnemonic(a),
                    Self::mnemonic(b),
                    Self::mnemonic(c),
                    Self::mnemonic(d)
                );

                let time = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);

                let split = time.to_be_bytes();
                let av = u32::from_be_bytes([split[3], split[2], split[1], split[0]]);
                let bv = u32::from_be_bytes([split[7], split[6], split[5], split[4]]);
                let cv = u32::from_be_bytes([split[11], split[10], split[9], split[8]]);
                let dv = u32::from_be_bytes([split[15], split[14], split[13], split[12]]);

                self.gp.set_reg(a, dv)?;
                self.gp.set_reg(b, cv)?;
                self.gp.set_reg(c, bv)?;
                self.gp.set_reg(d, av)?;
            }

            // rdpc {reg}
            (0, dst, 0x5, _, _, None) => {
                trace!("rdpc {}", Self::mnemonic(dst));
                self.gp.set_reg(dst, self.pc)?;
            }

            // kbrd {reg}
            (0, dst, 0x6, _, _, None) => {
                trace!("kbrd {}", Self::mnemonic(dst));
                unimplemented!()
            }

            // setgfx {reg} / setgfx {imm}
            (0, _, 0x7, a, _, imm) => {
                match imm {
                    Some(i) => trace!("setgfx 0x{i:0>8x}"),
                    None => trace!("setgfx {}", Self::mnemonic(a)),
                }

                let a = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(a)?,
                };

                self.gfx = a;
            }

            // draw
            (0, _, 0x8, _, _, _) => {
                trace!("draw");
                unimplemented!()
            }

            // sleep {reg}, {reg} / sleep {imm}
            (0, _, 0x9, a, b, imm) => {
                match imm {
                    Some(i) => trace!("sleep {i}"),
                    None => trace!("sleep {}, {}", Self::mnemonic(a), Self::mnemonic(b)),
                }

                let val = match imm {
                    Some(i) => i as _,
                    None => {
                        let val = self.gp.get_reg(b)?.to_be_bytes();
                        let val2 = self.gp.get_reg(a)?.to_be_bytes();

                        u64::from_be_bytes([
                            val2[3], val2[2], val2[1], val2[0], val[3], val[2], val[1], val[0],
                        ])
                    }
                };

                thread::sleep(Duration::from_micros(val));
            }

            // TODO: load / str ops

            // nand {reg}, {reg}, {reg}
            (1, dst, 0x0, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "nand {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "nand {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let a = self.gp.get_reg(a)?;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                self.gp.set_reg(dst, !(a & b))?;
            }

            // or {reg}, {reg}, {reg}
            (1, dst, 0x1, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "or {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "or {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let a = self.gp.get_reg(a)?;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                self.gp.set_reg(dst, a | b)?;
            }

            // cmp {reg}, {reg}, {reg} / cmp {reg}, {reg}, {imm}
            (1, dst, 0x0a, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "cmp {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "cmp {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let a = self.gp.get_reg(a)?;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let zero = a == b;
                let less_unsigned = a < b;
                let less_signed = (a as i32) < b as i32;
                let overflow = (a as i32).checked_sub(b as i32).is_none();

                let mut flags = Flags::empty();

                if zero {
                    flags |= Flags::Z;
                }

                if less_unsigned {
                    flags |= Flags::C;
                }

                if less_signed {
                    flags |= Flags::S;
                }

                if overflow {
                    flags |= Flags::O;
                }

                self.gp.set_reg(dst, flags.bits())?;
            }

            //
            // CONDITIONALS
            //

            // jmp {reg} / jmp {imm}
            (2, dst, 0x0, _, _, imm) => {
                match imm {
                    Some(i) => trace!("jmp 0x{i:0>8x}"),
                    None => trace!("jmp {}", Self::mnemonic(dst)),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };

                self.pc = dst;
                return Ok(());
            }

            // je {reg}, {reg} / je {reg}, {imm}
            (2, _, 0x1, f, b, imm) => {
                match imm {
                    Some(i) => trace!("je {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("je {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                if flags.contains(Flags::Z) {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jne {reg}, {reg} / jne {reg}, {imm}
            (2, _, 0x2, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jne {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jne {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                if !flags.contains(Flags::Z) {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jl {reg}, {reg} / jl {reg}, {imm}
            (2, _, 0x3, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jl {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jl {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let sf = flags.contains(Flags::S);
                let of = flags.contains(Flags::O);

                if sf != of {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jge {reg}, {reg} / jge {reg}, {imm}
            (2, _, 0x4, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jge {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jge {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let sf = flags.contains(Flags::S);
                let of = flags.contains(Flags::O);

                if sf == of {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jle {reg}, {reg} / jle {reg}, {imm}
            (2, _, 0x5, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jle {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jle {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let zf = flags.contains(Flags::Z);
                let sf = flags.contains(Flags::S);
                let of = flags.contains(Flags::O);

                if zf || (sf != of) {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jg {reg}, {reg} / jg {reg}, {imm}
            (2, _, 0x6, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jg {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jg {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let zf = flags.contains(Flags::Z);
                let sf = flags.contains(Flags::S);
                let of = flags.contains(Flags::O);

                if !zf && (sf == of) {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jb {reg}, {reg} / jb {reg}, {imm}
            (2, _, 0x7, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jb {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jb {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let cf = flags.contains(Flags::C);

                if cf {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jae {reg}, {reg} / jae {reg}, {imm}
            (2, _, 0x8, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jae {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jae {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let cf = flags.contains(Flags::C);

                if !cf {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jbe {reg}, {reg} / jbe {reg}, {imm}
            (2, _, 0x9, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jbe {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jbe {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let zf = flags.contains(Flags::Z);
                let cf = flags.contains(Flags::C);

                if cf || zf {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // ja {reg}, {reg} / ja {reg}, {imm}
            (2, _, 0xa, f, b, imm) => {
                match imm {
                    Some(i) => trace!("ja {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("ja {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let zf = flags.contains(Flags::Z);
                let cf = flags.contains(Flags::C);

                if !cf && !zf {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // js {reg}, {reg} / js {reg}, {imm}
            (2, _, 0xb, f, b, imm) => {
                match imm {
                    Some(i) => trace!("js {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("js {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let sf = flags.contains(Flags::S);

                if sf {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jo {reg}, {reg} / jo {reg}, {imm}
            (2, _, 0xc, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jo {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jo {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let of = flags.contains(Flags::O);

                if of {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jns {reg}, {reg} / jns {reg}, {imm}
            (2, _, 0xd, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jns {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jns {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let sf = flags.contains(Flags::S);

                if !sf {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            // jno {reg}, {reg} / jno {reg}, {imm}
            (2, _, 0xe, f, b, imm) => {
                match imm {
                    Some(i) => trace!("jno {}, 0x{i:0>8x}", Self::mnemonic(f)),
                    None => trace!("jno {}, {}", Self::mnemonic(f), Self::mnemonic(b)),
                }

                let f = self.gp.get_reg(f)?;
                let flags = Flags::from_bits_retain(f);

                let jmp_addr = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                };

                let of = flags.contains(Flags::O);

                if !of {
                    self.pc = jmp_addr;
                    return Ok(());
                }
            }

            //
            // STACK
            //
            _ => unimplemented!(),
        }

        self.pc += if inst.imm.is_some() { 8 } else { 4 };

        Ok(())
    }

    fn mnemonic(reg: u8) -> &'static str {
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

            _ => unreachable!("can't log mnemonic. this is a bug"),
        }
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

impl Deref for Registers {
    type Target = [BitSize];

    fn deref(&self) -> &Self::Target {
        const {
            assert!(
                size_of::<Self>().is_multiple_of(size_of::<BitSize>()),
                "Registers size does not fit evenly"
            );
        }

        let slice = slice::from_ref(self);
        bytemuck::must_cast_slice(slice)
    }
}

impl DerefMut for Registers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        const {
            assert!(
                size_of::<Self>().is_multiple_of(size_of::<BitSize>()),
                "Registers size does not fit evenly"
            );
        }

        let slice = slice::from_mut(self);
        bytemuck::must_cast_slice_mut(slice)
    }
}

impl Registers {
    /// Set register based on index
    #[inline]
    pub fn set_reg(&mut self, reg: u8, val: BitSize) -> Result<(), RegError> {
        let elem = self.get_mut(reg as usize).ok_or(RegError(reg))?;

        *elem = val;

        Ok(())
    }

    /// Read register based on index
    #[inline]
    pub fn get_reg(&self, reg: u8) -> Result<BitSize, RegError> {
        let elem = *self.get(reg as usize).ok_or(RegError(reg))?;

        Ok(elem)
    }
}
