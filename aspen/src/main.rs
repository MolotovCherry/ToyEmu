mod cpu;
mod emulator;
mod instruction;
mod memory;

use std::{env, error::Error, fs, process};

use env_logger::Env;

use crate::emulator::Emulator;

pub type BitSize = u32;

fn main() -> Result<(), Box<dyn Error>> {
    let env = Env::default().filter_or("EMU_LOG", "warn");
    env_logger::Builder::from_env(env)
        .format_timestamp(None)
        .init();

    let Some(file) = env::args().nth(1) else {
        eprintln!("aspen <file>");
        return Ok(());
    };

    let program = fs::read(file)?;
    let mut emu = Emulator::new(&program)?;

    match emu.run() {
        // I'd love to use ExitCode, but ExitCode supporting
        // u32 is nightly only for Windows
        Ok(code) => process::exit(code as _),
        Err(e) => eprintln!("{e}"),
    }

    Ok(())
}
