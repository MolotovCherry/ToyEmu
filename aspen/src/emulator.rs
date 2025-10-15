#[cfg(test)]
mod tests;

#[cfg(feature = "steady-clock")]
use std::time::{Duration, Instant};

use log::{Level, trace};
use yansi::Paint as _;

use crate::BitSize;
use crate::cpu::{Cpu, CpuError};
use crate::instruction::{InstError, Instruction};
use crate::memory::{MemError, Memory, PAGE_SIZE, Prot};
#[cfg(feature = "steady-clock")]
use crate::sleep::u_sleep;

#[cfg(feature = "steady-clock")]
pub const FREQ: Duration = Duration::from_micros(5);

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum EmuError {
    #[error("{0}")]
    Mem(#[from] MemError),
    #[error("{0}")]
    Inst(#[from] InstError),
    #[error("{0}")]
    Cpu(#[from] CpuError),
}

#[derive(Debug)]
pub struct Emulator {
    pub cpu: Cpu,
    pub mem: Memory,
}

impl Emulator {
    pub fn new(program: &[u8]) -> Result<Self, EmuError> {
        let mut this = Self {
            cpu: Cpu::new(),
            mem: Memory::new()?,
        };

        this.write_program(program)?;

        Ok(this)
    }

    pub fn write_program(&mut self, program: &[u8]) -> Result<(), MemError> {
        self.mem[..program.len() as BitSize].copy_from_slice(program);
        let size = program.len().next_multiple_of(PAGE_SIZE);
        self.mem
            .change_prot(..size as u32, Prot::Execute | Prot::Read)?;

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), EmuError> {
        let mut stop = false;

        loop {
            let mut clk = 1u32;
            let inst = self.next_inst()?;

            self.mem.check_prot(self.cpu.pc, Prot::Execute.into())?;

            if log::log_enabled!(Level::Trace) {
                #[cold]
                fn trace(pc: u32, i: &Instruction) {
                    trace!(target: "aspen::cpu", "{}: {i}", format_args!("0x{pc:0>8x}").bright_green());
                }

                trace(self.cpu.pc, &inst);
            }

            #[cfg(feature = "steady-clock")]
            let now = Instant::now();

            self.cpu.process(inst, &mut self.mem, &mut stop, &mut clk)?;

            #[cfg(feature = "steady-clock")]
            let elapsed = now.elapsed();

            #[rustfmt::skip]
            if stop { break; };

            #[cfg(feature = "steady-clock")]
            {
                let wait = FREQ * clk;
                if elapsed < wait {
                    u_sleep(wait - elapsed);
                }
            }

            // clock cycles we've been powered on for
            self.cpu.clk += clk as u64;
        }

        Ok(())
    }

    fn next_inst(&self) -> Result<Instruction, InstError> {
        let view = &self.mem[self.cpu.pc..];
        let i = Instruction::from_slice(view)?;

        Ok(i)
    }
}
