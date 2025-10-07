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

use crate::BitSize;

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum InstError {
    #[error("Incorrect instruction size: {0}")]
    WrongSize(usize),
}

pub struct Instruction {
    pub mode: u8,
    pub dst: u8,
    pub op_code: u8,
    pub a: u8,
    pub b: u8,
    pub imm: Option<BitSize>,
}

impl Instruction {
    pub fn from_slice(inst: &[u8]) -> Result<Self, InstError> {
        if inst.len() < 4 {
            return Err(InstError::WrongSize(inst.len()));
        }

        let ctrl = inst[0];
        let op_code = inst[1];
        let a = inst[2];
        let b = inst[3];

        let mode = ctrl.rotate_left(2);
        let is_imm = ((ctrl >> 5) & 0b001) == 1;
        let dst = ctrl & 0b00011111;

        let imm = if is_imm {
            if inst.len() < 8 {
                return Err(InstError::WrongSize(inst.len()));
            }

            let arr: [u8; 4] = inst[4..8].try_into().unwrap();
            let val = BitSize::from_le_bytes(arr);

            Some(val)
        } else {
            None
        };

        let this = Self {
            mode,
            dst,
            op_code,
            a,
            b,
            imm,
        };

        Ok(this)
    }
}
