use crate::BitSize;
use crate::cpu::Cpu;
use crate::memory::{MemError, Memory};

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum EmuError {
    #[error("{0}")]
    Mem(#[from] MemError),
}

pub struct Emulator {
    pub cpu: Cpu,
    pub mem: Memory,
}

impl Emulator {
    pub fn new(program: &[u8]) -> Result<Self, EmuError> {
        let mut mem = Memory::new()?;
        mem.view_mut(..program.len() as BitSize)
            .copy_from_slice(program);

        let this = Self {
            cpu: Cpu::default(),
            mem,
        };

        Ok(this)
    }

    pub fn run(&mut self) {
        loop {
            
        }
    }
    
    pub fn next_inst(&mut self) {
        
    }
}
