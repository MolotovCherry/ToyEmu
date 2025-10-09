use std::{slice, time::SystemTime};

use bstr::ByteSlice;
use bytemuck::{AnyBitPattern, NoUninit};

use crate::{BitSize, instruction::Instruction, memory::Memory};

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

macro_rules! trace {
  ($($args:tt)*) => {
    if log::log_enabled!(log::Level::Trace) {
      ({
        #[cold]
        #[inline(never)]
        || log::trace!($($args)*)
      })();
    }
  }
}

impl Cpu {
    pub fn process(
        &mut self,
        inst: Instruction,
        mem: &mut Memory,
        stop: &mut bool,
        clk: &mut u32,
    ) -> Result<(), CpuError> {
        match (inst.mode, inst.dst, inst.op_code, inst.a, inst.b, inst.imm) {
            // nop
            (0, _, 0x00, _, _, None) => trace!("nop"),

            // hlt
            (0, _, 0x01, _, _, None) => {
                trace!("hlt");
                *stop = true;
                return Ok(());
            }

            // pr {reg}, {reg}
            (0, _, 0x02, a, b, _) => {
                trace!("pr {}, {}", Self::mnemonic(a), Self::mnemonic(b));

                let low = self.gp.get_reg(a)?;
                let high = self.gp.get_reg(b)?;

                if let Some(view) = mem.view(low..high) {
                    let data = view.as_bstr();
                    print!("{data}");
                }
            }

            // epr {reg}, {reg}
            (0, _, 0x03, a, b, _) => {
                trace!("epr {}, {}", Self::mnemonic(a), Self::mnemonic(b));

                let low = self.gp.get_reg(a)?;
                let high = self.gp.get_reg(b)?;

                let data = mem[low..high].as_bstr();
                eprint!("{data}");
            }

            // time {reg}, {reg}, {reg}, {reg}
            (0, _, 0x04, a, b, Some(i)) => {
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
            (0, dst, 0x05, _, _, None) => {
                trace!("rdpc {}", Self::mnemonic(dst));
                self.gp.set_reg(dst, self.pc)?;
            }

            // kbrd {reg}
            (0, dst, 0x06, _, _, None) => {
                trace!("kbrd {}", Self::mnemonic(dst));
                unimplemented!()
            }

            // setgfx {reg} / setgfx {imm}
            (0, _, 0x07, a, _, imm) => {
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
            (0, _, 0x08, _, _, _) => {
                trace!("draw");
                unimplemented!()
            }

            // slp {reg}, {reg} / slp {imm}
            (0, _, 0x09, a, b, imm) => {
                match imm {
                    Some(i) => trace!("slp {i}"),
                    None => trace!("slp {}, {}", Self::mnemonic(a), Self::mnemonic(b)),
                }

                let val = match imm {
                    Some(i) => i as _,
                    None => {
                        let val = self.gp.get_reg(b)?.to_be_bytes();
                        let val2 = self.gp.get_reg(a)?.to_be_bytes();

                        u64::from_be_bytes([
                            val2[0], val2[1], val2[2], val2[3], val[0], val[1], val[2], val[3],
                        ])
                    }
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

            // rdclk {reg}, {reg}
            (0, _, 0x0a, a, b, _) => {
                trace!("rdclk {}, {}", Self::mnemonic(a), Self::mnemonic(b));

                let val = self.clk.to_be_bytes();
                let high = BitSize::from_be_bytes([val[0], val[1], val[2], val[3]]);
                let low = BitSize::from_be_bytes([val[4], val[5], val[6], val[7]]);

                self.gp.set_reg(a, high)?;
                self.gp.set_reg(b, low)?;
            }

            // TODO: load / str ops

            //
            // MATH
            //

            // nand {reg}, {reg}, {reg} / nand {reg}, {reg}, {imm}
            (1, dst, 0x00, a, b, imm) => {
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

            // or {reg}, {reg}, {reg} / or {reg}, {reg}, {imm}
            (1, dst, 0x01, a, b, imm) => {
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

            // and {reg}, {reg}, {reg} / and {reg}, {reg}, {imm}
            (1, dst, 0x02, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "and {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "and {}, {}, {}",
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

                self.gp.set_reg(dst, a & b)?;
            }

            // nor {reg}, {reg}, {reg} / nor {reg}, {reg}, {imm}
            (1, dst, 0x03, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "nor {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "nor {}, {}, {}",
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

                self.gp.set_reg(dst, !(a | b))?;
            }

            // add {reg}, {reg}, {reg} / add {reg}, {reg}, {imm}
            (1, dst, 0x04, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "add {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "add {}, {}, {}",
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

                self.gp.set_reg(dst, a.wrapping_add(b))?;
            }

            // sub {reg}, {reg}, {reg} / sub {reg}, {reg}, {imm}
            (1, dst, 0x05, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "sub {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "sub {}, {}, {}",
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

                self.gp.set_reg(dst, a.wrapping_sub(b))?;
            }

            // xor {reg}, {reg}, {reg} / xor {reg}, {reg}, {imm}
            (1, dst, 0x06, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "xor {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "xor {}, {}, {}",
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

                self.gp.set_reg(dst, a ^ b)?;
            }

            // lsl {reg}, {reg}, {reg} / lsl {reg}, {reg}, {imm}
            (1, dst, 0x07, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "lsl {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "lsl {}, {}, {}",
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

                self.gp.set_reg(dst, a << b)?;
            }

            // lsr {reg}, {reg}, {reg} / lsr {reg}, {reg}, {imm}
            (1, dst, 0x08, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "lsr {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "lsr {}, {}, {}",
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

                self.gp.set_reg(dst, a >> b)?;
            }

            // mul {reg}, {reg}, {reg} / mul {reg}, {reg}, {imm}
            (1, dst, 0x09, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "mul {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "mul {}, {}, {}",
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

                self.gp.set_reg(dst, a.wrapping_mul(b))?;
            }

            // imul {reg}, {reg}, {reg} / imul {reg}, {reg}, {imm}
            (1, dst, 0x0a, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "imul {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "imul {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let a = self.gp.get_reg(a)? as i32;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                } as i32;

                self.gp.set_reg(dst, a.wrapping_mul(b) as u32)?;
            }

            // div {reg}, {reg}, {reg} / div {reg}, {reg}, {imm}
            (1, dst, 0x0b, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "div {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "div {}, {}, {}",
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

                let val = if a != 0 { a.wrapping_div(b) } else { 0 };

                self.gp.set_reg(dst, val)?;
            }

            // idiv {reg}, {reg}, {reg} / idiv {reg}, {reg}, {imm}
            (1, dst, 0x0c, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "idiv {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "idiv {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let a = self.gp.get_reg(a)? as i32;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                } as i32;

                let val = if a != 0 { a.wrapping_div(b) } else { 0 };

                self.gp.set_reg(dst, val as u32)?;
            }

            // rem {reg}, {reg}, {reg} / rem {reg}, {reg}, {imm}
            (1, dst, 0x0d, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "rem {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "rem {}, {}, {}",
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

                self.gp.set_reg(dst, a % b)?;
            }

            // irem {reg}, {reg}, {reg} / rem {reg}, {reg}, {imm}
            (1, dst, 0x0e, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "irem {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "irem {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let a = self.gp.get_reg(a)? as i32;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                } as i32;

                self.gp.set_reg(dst, (a % b) as u32)?;
            }

            // mov {reg}, {reg} / mov {reg}, {imm}
            (1, dst, 0x0f, a, _, imm) => {
                match imm {
                    Some(i) => trace!("mov {}, 0x{i:0>8x}", Self::mnemonic(dst)),
                    None => trace!("mov {}, {}", Self::mnemonic(dst), Self::mnemonic(a)),
                }

                let a = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(a)?,
                };

                self.gp.set_reg(dst, a)?;
            }

            // inc {reg}
            (1, dst, 0x10, a, _, _) => {
                trace!("inc {}", Self::mnemonic(dst));

                let a = self.gp.get_reg(a)?.wrapping_add(1);

                self.gp.set_reg(dst, a)?;
            }

            // dec {reg}
            (1, dst, 0x11, a, _, _) => {
                trace!("dec {}", Self::mnemonic(dst));

                let a = self.gp.get_reg(a)?.wrapping_sub(1);

                self.gp.set_reg(dst, a)?;
            }

            // se {reg}, {reg}, {reg} / se {reg}, {reg}, {imm}
            (1, dst, 0x12, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "se {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "se {}, {}, {}",
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

                self.gp.set_reg(dst, (a == b) as _)?;
            }

            // sne {reg}, {reg}, {reg} / sne {reg}, {reg}, {imm}
            (1, dst, 0x13, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "sne {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "sne {}, {}, {}",
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

                self.gp.set_reg(dst, (a != b) as _)?;
            }

            // sl {reg}, {reg}, {reg} / sl {reg}, {reg}, {imm}
            (1, dst, 0x14, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "sl {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "sl {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let a = self.gp.get_reg(a)? as i32;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                } as i32;

                self.gp.set_reg(dst, (a < b) as _)?;
            }

            // sle {reg}, {reg}, {reg} / sle {reg}, {reg}, {imm}
            (1, dst, 0x15, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "sle {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "sle {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let a = self.gp.get_reg(a)? as i32;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                } as i32;

                self.gp.set_reg(dst, (a <= b) as _)?;
            }

            // sg {reg}, {reg}, {reg} / sg {reg}, {reg}, {imm}
            (1, dst, 0x16, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "sg {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "sg {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let a = self.gp.get_reg(a)? as i32;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                } as i32;

                self.gp.set_reg(dst, (a > b) as _)?;
            }

            // sge {reg}, {reg}, {reg} / sge {reg}, {reg}, {imm}
            (1, dst, 0x17, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "sge {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "sge {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let a = self.gp.get_reg(a)? as i32;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                } as i32;

                self.gp.set_reg(dst, (a >= b) as _)?;
            }

            // asr {reg}, {reg}, {reg} / asr {reg}, {reg}, {imm}
            (1, dst, 0x18, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "asr {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a)
                    ),

                    None => trace!(
                        "asr {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                // rust performs arithmetic shift right
                // when args are signed
                let a = self.gp.get_reg(a)? as i32;
                let b = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(b)?,
                } as i32;

                self.gp.set_reg(dst, (a >> b) as u32)?;
            }

            //
            // CONDITIONALS
            //

            // jmp {reg}, {reg}, {reg} / jmp {reg}, {reg}, {imm}
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

            // je {reg}, {reg}, {reg} / je {reg}, {reg}, {imm}
            (2, dst, 0x1, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "je {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),

                    None => trace!(
                        "je {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };
                let a = self.gp.get_reg(a)?;
                let b = self.gp.get_reg(b)?;

                if a == b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            // jne {reg}, {reg}, {reg} / jne {reg}, {reg}, {imm}
            (2, dst, 0x2, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "jne {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),

                    None => trace!(
                        "jne {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };
                let a = self.gp.get_reg(a)?;
                let b = self.gp.get_reg(b)?;

                if a != b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            // jl {reg}, {reg}, {reg} / jl {reg}, {reg}, {imm}
            (2, dst, 0x3, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "jl {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),

                    None => trace!(
                        "jl {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };
                let a = self.gp.get_reg(a)? as i32;
                let b = self.gp.get_reg(b)? as i32;

                if a < b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            // jge {reg}, {reg}, {reg} / jge {reg}, {reg}, {imm}
            (2, dst, 0x4, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "jge {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),

                    None => trace!(
                        "jge {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };
                let a = self.gp.get_reg(a)? as i32;
                let b = self.gp.get_reg(b)? as i32;

                if a >= b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            // jle {reg}, {reg}, {reg} / jle {reg}, {reg}, {imm}
            (2, dst, 0x5, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "jle {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),

                    None => trace!(
                        "jle {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };
                let a = self.gp.get_reg(a)? as i32;
                let b = self.gp.get_reg(b)? as i32;

                if a <= b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            // jg {reg}, {reg}, {reg} / jg {reg}, {reg}, {imm}
            (2, dst, 0x6, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "jg {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),

                    None => trace!(
                        "jg {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };
                let a = self.gp.get_reg(a)? as i32;
                let b = self.gp.get_reg(b)? as i32;

                if a > b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            // jb {reg}, {reg}, {reg} / jb {reg}, {reg}, {imm}
            (2, dst, 0x7, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "jb {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),

                    None => trace!(
                        "jb {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };
                let a = self.gp.get_reg(a)?;
                let b = self.gp.get_reg(b)?;

                if a < b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            // jae {reg}, {reg}, {reg} / jae {reg}, {reg}, {imm}
            (2, dst, 0x8, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "jae {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),

                    None => trace!(
                        "jae {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };
                let a = self.gp.get_reg(a)?;
                let b = self.gp.get_reg(b)?;

                if a >= b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            // jbe {reg}, {reg}, {reg} / jbe {reg}, {reg}, {imm}
            (2, dst, 0x9, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "jbe {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),

                    None => trace!(
                        "jbe {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };
                let a = self.gp.get_reg(a)?;
                let b = self.gp.get_reg(b)?;

                if a <= b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            // ja {reg}, {reg}, {reg} / ja {reg}, {reg}, {imm}
            (2, dst, 0xa, a, b, imm) => {
                match imm {
                    Some(i) => trace!(
                        "jb {}, {}, 0x{i:0>8x}",
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),

                    None => trace!(
                        "jb {}, {}, {}",
                        Self::mnemonic(dst),
                        Self::mnemonic(a),
                        Self::mnemonic(b)
                    ),
                }

                let dst = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(dst)?,
                };
                let a = self.gp.get_reg(a)?;
                let b = self.gp.get_reg(b)?;

                if a > b {
                    self.pc = dst;
                    return Ok(());
                }
            }

            //
            // STACK
            //

            // push {reg}
            (3, _, 0x0, a, _, _) => {
                trace!("push {}", Self::mnemonic(a));

                let a = self.gp.get_reg(a)?;
                let old_sp = self.gp.sp;

                self.gp.sp = self.gp.sp.wrapping_sub(size_of::<BitSize>() as _);

                let slice = if old_sp < self.gp.sp {
                    &mut mem[self.gp.sp..]
                } else {
                    &mut mem[self.gp.sp..old_sp]
                };

                slice.copy_from_slice(&a.to_be_bytes());

                *clk = 2;
            }

            // pop {reg}
            (3, dst, 0x1, _, _, _) => {
                trace!("pop {}", Self::mnemonic(dst));

                let bytes = &mem[self.gp.sp..self.gp.sp + size_of::<BitSize>() as BitSize];
                let data = BitSize::from_be_bytes(bytes.try_into().unwrap());
                self.gp.sp = self.gp.sp.wrapping_add(size_of::<BitSize>() as _);
                self.gp.set_reg(dst, data)?;

                *clk = 2;
            }

            // call {reg}, call {imm}
            (3, _, 0x2, a, _, imm) => {
                match imm {
                    Some(i) => trace!("call 0x{i:0>8x}"),
                    None => trace!("call {}", Self::mnemonic(a)),
                }

                // push old ra to stack
                let old_sp = self.gp.sp;
                self.gp.sp = self.gp.sp.wrapping_sub(size_of::<BitSize>() as _);
                mem[self.gp.sp..old_sp].copy_from_slice(&self.gp.ra.to_be_bytes());

                let jmp = match imm {
                    Some(i) => i,
                    None => self.gp.get_reg(a)?,
                };

                // return back to current pc
                self.gp.ra = self.pc;
                // make sure to set to next instruction
                self.gp.ra += if inst.imm.is_some() { 8 } else { 4 };

                // set pc to new loc
                self.pc = jmp;

                *clk = 3;

                return Ok(());
            }

            // ret
            (3, _, 0x3, _, _, _) => {
                trace!("ret");

                // jmp to return addr
                self.pc = self.gp.ra;

                // pop old ra off stack and set it
                let bytes = &mem[self.gp.sp..self.gp.sp + size_of::<BitSize>() as BitSize];
                let ra = BitSize::from_be_bytes(bytes.try_into().unwrap());
                self.gp.sp = self.gp.sp.wrapping_add(size_of::<BitSize>() as _);
                self.gp.ra = ra;

                *clk = 2;

                return Ok(());
            }

            _ => return Err(CpuError::UnsupportedInst(inst)),
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

            _ => "<unkn>",
        }
    }

    /// zero all registers
    #[allow(unused)]
    pub fn zeroize(&mut self) {
        self.gfx = 0;
        self.pc = 0;
        self.gp = Registers::default();
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
}
