// M = Mode
// O = Opcode
// I = 0/1 Is argument B an immediate value bit
// D = Destination register
// A = Argument A
// B = Argument B
// Z = Immediate value
//
// Instruction encoding if not an immediate value:
// MMIDDDDD OOOOOOOO AAAAAAAA BBBBBBBB
// Instruction encoding if an immediate value:
// MMIDDDDD OOOOOOOO AAAAAAAA BBBBBBBB ZZZZZZZZ ZZZZZZZZ ZZZZZZZZ ZZZZZZZZ

use std::fmt::Display;

use strum::Display;

use crate::BitSize;

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum InstError {
    #[error("Incorrect instruction size: {0}")]
    WrongSize(usize),
    #[error("Unknown opcode: {0}")]
    UnknownOpcode(u8),
}

#[derive(Debug, Copy, Clone)]
pub struct Instruction {
    pub ty: InstructionType,
    pub dst: u8,
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8,
    pub has_imm: bool,
    pub imm: BitSize,
}

impl Instruction {
    pub fn from_slice(inst: &[u8]) -> Result<Self, InstError> {
        // note: slice comes in unbounded on the right side
        // slice it to possible full instruction length
        let inst = &inst[..inst.len().min(8)];

        if inst.len() < 4 {
            return Err(InstError::WrongSize(inst.len()));
        }

        let ctrl = inst[0];
        let opcode = inst[1];
        let a = inst[2];
        let b = inst[3];

        // these are all in BE
        let mode = ctrl.rotate_left(2) & 0b11;
        let has_imm = ((ctrl >> 5) & 0b1) == 1;
        let dst = ctrl & 0b11111;

        let mut c = 0;
        let mut d = 0;
        let mut e = 0;
        let mut f = 0;

        // imm is in LE
        let imm = if has_imm {
            if inst.len() < 8 {
                return Err(InstError::WrongSize(inst.len()));
            }

            [c, d, e, f] = inst[4..8].try_into().unwrap();
            BitSize::from_le_bytes([c, d, e, f])
        } else {
            Default::default()
        };

        let ty = InstructionType::try_from(mode, opcode).ok_or(InstError::UnknownOpcode(opcode))?;

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
        write!(f, "{}", self.ty)?;

        let args = self.ty.args();

        let should_print_imm = self.has_imm
            && !args
                .iter()
                .any(|i| matches!(i, Register::C | Register::D | Register::E | Register::F));

        for (i, arg) in args.iter().enumerate() {
            let reg = match arg {
                Register::Dst => self.dst,
                Register::A => self.a,
                Register::B => self.b,

                Register::C if !should_print_imm => (self.imm >> 24) as _,

                Register::D if !should_print_imm => (self.imm >> 16) as _,

                Register::E if !should_print_imm => (self.imm >> 8) as _,

                Register::F if !should_print_imm => self.imm as u8,

                Register::Imm if should_print_imm => {
                    if i > 0 {
                        write!(f, ", 0x{i:0>8x}")?;
                    } else {
                        write!(f, " 0x{i:0>8x}")?;
                    }

                    continue;
                }

                _ => continue,
            };

            if i > 0 {
                write!(f, ", {}", mnemonic(reg))?;
            } else {
                write!(f, " {}", mnemonic(reg))?;
            }
        }

        Ok(())
    }
}

#[expect(dead_code)]
#[derive(Copy, Clone, Debug)]
enum Register {
    Dst,
    A,
    B,
    C,
    D,
    E,
    F,
    Imm,
}

macro_rules! impl_inst {
    (
        $(
            $(#[$m:meta])*
            ($mode:expr, $opcode:expr) => $inst:ident $([$($op:ident),*])?
        )+
    ) => {
        #[derive(Copy, Clone, Debug, Display)]
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

            fn args(&self) -> &'static [Register] {
                match self {
                    $(
                        Self::$inst => &[$($(Register::$op,)*)*],
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
    (0, 0x07) => Setgfx [A, Imm]
    (0, 0x08) => Draw
    (0, 0x09) => Slp [A, B, Imm]
    (0, 0x0a) => Rdclk [A, B]

    // Memory
    (0, 0x20) => Ld
    #[strum(to_string = "ld.w")]
    (0, 0x21) => Ldw
    #[strum(to_string = "ld.b")]
    (0, 0x22) => Ldb

    (0, 0x23) => Pld
    #[strum(to_string = "pld.w")]
    (0, 0x24) => Pldw
    #[strum(to_string = "pld.b")]
    (0, 0x25) => Pldb

    (0, 0x26) => Str
    #[strum(to_string = "str.w")]
    (0, 0x27) => Strw
    #[strum(to_string = "str.b")]
    (0, 0x28) => Strb

    (0, 0x29) => Pstr
    #[strum(to_string = "pstr.w")]
    (0, 0x2a) => Pstrw
    #[strum(to_string = "pstr.b")]
    (0, 0x2b) => Pstrb

    // Math
    (1, 0x00) => Nand [Dst, A, B, Imm]
    (1, 0x01) => Or [Dst, A, B, Imm]
    (1, 0x02) => And [Dst, A, B, Imm]
    (1, 0x03) => Nor [Dst, A, B, Imm]
    (1, 0x04) => Add [Dst, A, B, Imm]
    (1, 0x05) => Sub [Dst, A, B, Imm]
    (1, 0x06) => Xor [Dst, A, B, Imm]
    (1, 0x07) => Lsl [Dst, A, B, Imm]
    (1, 0x08) => Lsr [Dst, A, B, Imm]
    (1, 0x09) => Mul [Dst, A, B, Imm]
    (1, 0x0a) => Imul [Dst, A, B, Imm]
    (1, 0x0b) => Div [Dst, A, B, Imm]
    (1, 0x0c) => Idiv [Dst, A, B, Imm]
    (1, 0x0d) => Rem [Dst, A, B, Imm]
    (1, 0x0e) => Irem [Dst, A, B, Imm]
    (1, 0x0f) => Mov [Dst, A, Imm]
    (1, 0x10) => Inc [Dst, A, B, Imm]
    (1, 0x11) => Dec [Dst, A, B, Imm]
    (1, 0x12) => Se [Dst, A, B, Imm]
    (1, 0x13) => Sne [Dst, A, B, Imm]
    (1, 0x14) => Sl [Dst, A, B, Imm]
    (1, 0x15) => Sle [Dst, A, B, Imm]
    (1, 0x16) => Sg [Dst, A, B, Imm]
    (1, 0x17) => Sge [Dst, A, B, Imm]
    (1, 0x18) => Asr [Dst, A, B, Imm]

    // Cond
    (2, 0x00) => Jmp [Dst, Imm]
    (2, 0x01) => Je [A, B, Dst, Imm]
    (2, 0x02) => Jne [A, B, Dst]
    (2, 0x03) => Jl [A, B, Dst]
    (2, 0x04) => Jge [A, B, Dst]
    (2, 0x05) => Jle [A, B, Dst]
    (2, 0x06) => Jg [A, B, Dst]
    (2, 0x07) => Jb [A, B, Dst]
    (2, 0x08) => Jae [A, B, Dst]
    (2, 0x09) => Jbe [A, B, Dst]
    (2, 0x0a) => Ja [A, B, Dst]

    // Stack
    (3, 0x00) => Push [A]
    (3, 0x01) => Pop [Dst]
    (3, 0x02) => Call [A, Imm]
    (3, 0x03) => Ret
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

        _ => "<un>",
    }
}
