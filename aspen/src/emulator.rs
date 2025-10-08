#[cfg(test)]
mod tests;

use std::sync::mpsc::{TryRecvError, channel};

use crate::BitSize;
use crate::cpu::{Cpu, CpuError};
use crate::instruction::{InstError, Instruction};
use crate::memory::{MemError, Memory};

#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum EmuError {
    #[error("{0}")]
    Mem(#[from] MemError),
    #[error("{0}")]
    Inst(#[from] InstError),
    #[error("{0}")]
    Cpu(#[from] CpuError),
}

pub struct Emulator {
    pub cpu: Cpu,
    pub mem: Memory,
}

impl Emulator {
    pub fn new(program: &[u8]) -> Result<Self, EmuError> {
        let mut this = Self {
            cpu: Cpu::default(),
            mem: Memory::new()?,
        };

        this.write_program(program);

        Ok(this)
    }

    fn write_program(&mut self, program: &[u8]) {
        self.mem[..program.len() as BitSize].copy_from_slice(program);
    }

    pub fn run(&mut self) -> Result<u32, EmuError> {
        let (stop_tx, stop_recv) = channel();

        let code = loop {
            let inst = self.next_inst()?;

            self.cpu.process(inst, &mut self.mem, &stop_tx)?;

            match stop_recv.try_recv() {
                Ok(c) => break c,
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => {
                    unreachable!("stop_tx unexpectedly disconnected")
                }
            }
        };

        Ok(code)
    }

    fn next_inst(&self) -> Result<Instruction, InstError> {
        let view = &self.mem[self.cpu.pc..];
        let i = Instruction::from_slice(view)?;

        Ok(i)
    }
}
