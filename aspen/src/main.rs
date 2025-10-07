mod cpu;
mod emulator;
mod instruction;
mod memory;

use std::{env, error::Error, fs};

use env_logger::Env;

use crate::emulator::Emulator;

pub type BitSize = u32;

fn main() -> Result<(), Box<dyn Error>> {
    let env = Env::default().filter_or("EMU_LOG", "warn");
    env_logger::Builder::from_env(env).init();

    let Some(file) = env::args().nth(1) else {
        eprintln!("aspen <file>");
        return Ok(());
    };

    let program = fs::read(file)?;
    let mut emu = Emulator::new(&program)?;
    emu.run()?;

    Ok(())
}
