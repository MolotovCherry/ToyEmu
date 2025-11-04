#[cfg(test)]
mod tests;

use std::sync::Arc;

use log::{Level, trace};
use memmap2::Mmap;
use yansi::Paint as _;

use crate::BitSize;
use crate::cpu::{Cpu, CpuError};
use crate::instruction::{InstError, Instruction};
use crate::mmu::{MemError, Mmu, PAGE_SIZE, Prot};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EmuError {
    #[error("{0}")]
    Mem(#[from] MemError),
    #[error("{0} @ 0x{1:08x}")]
    PageFault(MemError, BitSize),
    #[error("{0}")]
    Inst(#[from] InstError),
    #[error("{0}")]
    Cpu(#[from] CpuError),
}

#[derive(Debug)]
pub struct Emulator {
    pub cpu: Cpu,
    pub mmu: Arc<Mmu>,
}

impl Emulator {
    pub fn new(program: &[u8]) -> Result<Self, EmuError> {
        let this = Self {
            cpu: Cpu::new(),
            mmu: Arc::new(Mmu::new()?),
        };

        this.write_program(program)?;

        let next_page = program.len().next_multiple_of(PAGE_SIZE);
        this.mmu
            .set_prot(next_page as BitSize.., Prot::Read | Prot::Write);

        let file = std::fs::File::open(r"R:\build\rust\adsf\new.bin").unwrap();

        let mmap = unsafe { Mmap::map(&file).unwrap() };

        this.mmu.memwrite(0x2800, &mmap).unwrap();

        Ok(this)
    }

    pub fn write_program(&self, program: &[u8]) -> Result<(), MemError> {
        let len = program.len() as BitSize;
        self.mmu.memwrite(0, program)?;
        let size = len.next_multiple_of(PAGE_SIZE as BitSize);
        self.mmu.set_prot(..size, Prot::Execute | Prot::Read);

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), EmuError> {
        let mut stop = false;

        loop {
            let mut clk = 1u32;
            let inst = self.next_inst()?;

            if let Err(e) = self.mmu.check_prot(self.cpu.pc, Prot::Execute) {
                return Err(EmuError::PageFault(e, self.cpu.pc));
            }

            if log::log_enabled!(Level::Trace) {
                #[cold]
                fn trace(pc: u32, i: &Instruction) {
                    trace!(target: "aspen::cpu", "{}: {i}", format_args!("0x{pc:0>8x}").bright_green());
                }

                trace(self.cpu.pc, &inst);
            }

            self.cpu.process(inst, &self.mmu, &mut stop, &mut clk)?;

            #[rustfmt::skip]
            if stop { break; };

            // clock cycles we've been powered on for
            self.cpu.clk += clk as u64;
        }

        Ok(())
    }

    fn next_inst(&self) -> Result<Instruction, EmuError> {
        let mut buf = [0u8; 8];
        self.mmu.memcpy(self.cpu.pc, &mut buf)?;
        let i = Instruction::from_buf(buf)?;

        Ok(i)
    }
}
