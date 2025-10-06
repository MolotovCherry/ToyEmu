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

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum InstError {
    #[error("Incorrect instruction size: {0}")]
    WrongSize(usize)
}

pub struct Instruction {
    mode: u8,
    dst: u8,
    op_code: u8,
    a: u8,
    b: u8,
    imm: Option<u32>,
}

impl Instruction {
    pub fn decode(inst: &[u8]) -> Result<Self, InstError> {
        if inst.len() < 4 {
            return Err(InstError::WrongSize(inst.len()));
        }
        
        

        let one = in
    }
}
