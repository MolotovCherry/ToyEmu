// M = Mode
// O = Opcode
// I = 0/1 Is argument B an immediate value bit
// D = Destination register
// A = Argument A
// B = Argument B
// Z = Immediate value
//
// Instruction encoding if not an immediate value:
// MMIDDDDD OOOOOOOO 000AAAAA 000BBBBB
// Instruction encoding if an immediate value:
// MMIDDDDD OOOOOOOO 000AAAAA 000BBBBB ZZZZZZZZ ZZZZZZZZ ZZZZZZZZ ZZZZZZZZ

use std::fmt::Display;

use strum::Display;
use yansi::Paint as _;

use crate::{BitSize, cpu::Reg};

#[derive(Debug, Copy, Clone, thiserror::Error, PartialEq)]
pub enum InstError {
    #[error("Unknown opcode: {0} {0}")]
    UnknownInstruction(u8, u8),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Instruction {
    pub ty: InstructionType,
    pub dst: Reg,
    pub a: Reg,
    pub b: Reg,
    pub c: Reg,
    pub d: Reg,
    pub e: Reg,
    pub f: Reg,
    pub has_imm: bool,
    pub imm: BitSize,
}

impl Instruction {
    pub fn from_buf(inst: [u8; 8]) -> Result<Self, InstError> {
        let ctrl = inst[0];
        let opcode = inst[1];
        let a = Reg::from(inst[2]);
        let b = Reg::from(inst[3]);

        // these are all in BE
        let mode = ctrl.rotate_left(2) & 0b11;
        let has_imm = ((ctrl >> 5) & 0b1) == 1;
        let dst = Reg::from(ctrl); // this already strips 5 LSB

        let mut c = Reg::Zr;
        let mut d = Reg::Zr;
        let mut e = Reg::Zr;
        let mut f = Reg::Zr;

        // imm is in LE
        let imm = if has_imm {
            let imm = BitSize::from_le_bytes([inst[4], inst[5], inst[6], inst[7]]);

            // we only need 5 bits to guarantee the reg size
            c = Reg::from(inst[4]);
            d = Reg::from(inst[5]);
            e = Reg::from(inst[6]);
            f = Reg::from(inst[7]);

            imm
        } else {
            Default::default()
        };

        let ty = InstructionType::try_from(mode, opcode)
            .ok_or(InstError::UnknownInstruction(mode, opcode))?;

        let this = Self {
            ty,
            dst,
            a,
            b,
            c,
            d,
            e,
            f,
            has_imm,
            imm,
        };

        Ok(this)
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ty.bright_magenta())?;

        let args = self.ty.args();

        for args in args.iter() {
            #[rustfmt::skip]
            let args_has_imm = args.iter().any(|i| {
                matches!(i, RegOpts::C | RegOpts::D | RegOpts::E | RegOpts::F | RegOpts::Imm)
            });

            if (self.has_imm && !args_has_imm) || (!self.has_imm && args_has_imm) {
                continue;
            }

            let mut offset = 0;
            let mut use_brackets = false;
            for (i, arg) in args.iter().enumerate() {
                let reg = match arg {
                    RegOpts::Dst => self.dst,
                    RegOpts::A => self.a,
                    RegOpts::B => self.b,

                    RegOpts::C => Reg::from(self.imm >> 24),

                    RegOpts::D => Reg::from(self.imm >> 16),

                    RegOpts::E => Reg::from(self.imm >> 8),

                    RegOpts::F => Reg::from(self.imm),

                    RegOpts::Imm => {
                        if use_brackets {
                            if i.saturating_sub(offset) > 0 {
                                write!(
                                    f,
                                    ", [{}]",
                                    format_args!("0x{:0>8x}", self.imm).bright_yellow()
                                )?;
                            } else {
                                write!(
                                    f,
                                    " [{}]",
                                    format_args!("0x{:0>8x}", self.imm).bright_yellow()
                                )?;
                            }
                        } else if i.saturating_sub(offset) > 0 {
                            write!(
                                f,
                                ", {}",
                                format_args!("0x{:0>8x}", self.imm).bright_yellow()
                            )?;
                        } else {
                            write!(
                                f,
                                " {}",
                                format_args!("0x{:0>8x}", self.imm).bright_yellow()
                            )?;
                        }

                        use_brackets = false;
                        continue;
                    }

                    RegOpts::Brackets => {
                        offset += 1;
                        use_brackets = true;
                        continue;
                    }
                };

                if use_brackets {
                    if i.saturating_sub(offset) > 0 {
                        write!(f, ", [{}]", reg.bright_blue())?;
                    } else {
                        write!(f, " [{}]", reg.bright_blue())?;
                    }
                } else if i.saturating_sub(offset) > 0 {
                    write!(f, ", {}", reg.bright_blue())?;
                } else {
                    write!(f, " {}", reg.bright_blue())?;
                }

                use_brackets = false;
            }
        }

        Ok(())
    }
}

#[expect(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
enum RegOpts {
    Dst,
    A,
    B,
    C,
    D,
    E,
    F,
    Imm,
    // Special opt which places brackets around next arg
    Brackets,
}

macro_rules! impl_inst {
    (
        $(
            $(#[$m:meta])*
            ($mode:expr, $opcode:expr) => $inst:ident $([$($op:ident),*])*
        )+
    ) => {
        #[derive(Copy, Clone, Debug, Display, PartialEq)]
        #[strum(serialize_all = "lowercase")]
        pub enum InstructionType {
            $(
                $(#[$m])*
                $inst,
            )+
        }

        impl InstructionType {
            fn try_from(mode: u8, opcode: u8) -> Option<Self> {
                let val = match (mode, opcode) {
                    $(
                        ($mode, $opcode) => Self::$inst,
                    )+

                    _ => return None,
                };

                Some(val)
            }

            fn args(&self) -> &'static [&'static [RegOpts]] {
                match self {
                    $(
                        Self::$inst => &[$(&[$(RegOpts::$op,)*]),*],
                    )+
                }
            }
        }
    };
}

impl_inst! {
    // (mode, opcode)

    (0, 0x00) => Nop
    (0, 0x01) => Hlt
    (0, 0x02) => Pr [A, B]
    (0, 0x03) => Epr [A, B]
    (0, 0x04) => Tme [A, B, C, D]
    (0, 0x05) => Rdpc [Dst]
    (0, 0x06) => Kbrd [Dst]
    (0, 0x07) => Setgfx [A] [Imm]
    (0, 0x08) => Draw
    (0, 0x09) => Slp [A, B] [Imm]
    (0, 0x0a) => Rdclk [A, B]
    (0, 0x0b) => Dbg [A]

    // Memory
    (0, 0x20) => Ld [Dst, Brackets, A] [Dst, Brackets, Imm]
    #[strum(to_string = "ld.w")]
    (0, 0x21) => Ldw [Dst, Brackets, A] [Dst, Brackets, Imm]
    #[strum(to_string = "ld.b")]
    (0, 0x22) => Ldb [Dst, Brackets, A] [Dst, Brackets, Imm]

    (0, 0x23) => Pld [Dst, Brackets, A] [Dst, Brackets, Imm]
    #[strum(to_string = "pld.w")]
    (0, 0x24) => Pldw [Dst, Brackets, A] [Dst, Brackets, Imm]
    #[strum(to_string = "pld.b")]
    (0, 0x25) => Pldb [Dst, Brackets, A] [Dst, Brackets, Imm]

    (0, 0x26) => Str [Brackets, Dst, A] [Brackets, Dst, Imm]
    #[strum(to_string = "str.w")]
    (0, 0x27) => Strw [Brackets, Dst, A] [Brackets, Dst, Imm]
    #[strum(to_string = "str.b")]
    (0, 0x28) => Strb [Brackets, Dst, A] [Brackets, Dst, Imm]

    (0, 0x29) => Pstr [Brackets, Dst, A] [Brackets, Dst, Imm]
    #[strum(to_string = "pstr.w")]
    (0, 0x2a) => Pstrw [Brackets, Dst, A] [Brackets, Dst, Imm]
    #[strum(to_string = "pstr.b")]
    (0, 0x2b) => Pstrb [Brackets, Dst, A] [Brackets, Dst, Imm]

    // Math
    (1, 0x00) => Nand [Dst, A, B] [Dst, A, Imm]
    (1, 0x01) => Or [Dst, A, B] [Dst, A, Imm]
    (1, 0x02) => And [Dst, A, B] [Dst, A, Imm]
    (1, 0x03) => Nor [Dst, A, B] [Dst, A, Imm]
    (1, 0x04) => Add [Dst, A, B] [Dst, A, Imm]
    (1, 0x05) => Sub [Dst, A, B] [Dst, A, Imm]
    (1, 0x06) => Xor [Dst, A, B] [Dst, A, Imm]
    (1, 0x07) => Lsl [Dst, A, B] [Dst, A, Imm]
    (1, 0x08) => Lsr [Dst, A, B] [Dst, A, Imm]
    (1, 0x09) => Mul [Dst, A, B] [Dst, A, Imm]
    (1, 0x0a) => Imul [Dst, A, B] [Dst, A, Imm]
    (1, 0x0b) => Div [Dst, A, B] [Dst, A, Imm]
    (1, 0x0c) => Idiv [Dst, A, B] [Dst, A, Imm]
    (1, 0x0d) => Rem [Dst, A, B] [Dst, A, Imm]
    (1, 0x0e) => Irem [Dst, A, B] [Dst, A, Imm]
    (1, 0x0f) => Mov [Dst, A] [Dst, Imm]
    (1, 0x10) => Inc [Dst]
    (1, 0x11) => Dec [Dst]
    (1, 0x12) => Se [Dst, A, B] [Dst, A, Imm]
    (1, 0x13) => Sne [Dst, A, B] [Dst, A, Imm]
    (1, 0x14) => Sl [Dst, A, B] [Dst, A, Imm]
    (1, 0x15) => Sle [Dst, A, B] [Dst, A, Imm]
    (1, 0x16) => Sg [Dst, A, B] [Dst, A, Imm]
    (1, 0x17) => Sge [Dst, A, B] [Dst, A, Imm]
    (1, 0x18) => Asr [Dst, A, B] [Dst, A, Imm]

    // Cond
    (2, 0x00) => Jmp [Dst] [Imm]
    (2, 0x01) => Je [A, B, Dst] [A, B, Imm]
    (2, 0x02) => Jne [A, B, Dst] [A, B, Imm]
    (2, 0x03) => Jl [A, B, Dst] [A, B, Imm]
    (2, 0x04) => Jge [A, B, Dst] [A, B, Imm]
    (2, 0x05) => Jle [A, B, Dst] [A, B, Imm]
    (2, 0x06) => Jg [A, B, Dst] [A, B, Imm]
    (2, 0x07) => Jb [A, B, Dst] [A, B, Imm]
    (2, 0x08) => Jae [A, B, Dst] [A, B, Imm]
    (2, 0x09) => Jbe [A, B, Dst] [A, B, Imm]
    (2, 0x0a) => Ja [A, B, Dst] [A, B, Imm]

    // Stack
    (3, 0x00) => Push [A]
    (3, 0x01) => Pop [Dst]
    (3, 0x02) => Call [A] [Imm]
    (3, 0x03) => Ret
}
