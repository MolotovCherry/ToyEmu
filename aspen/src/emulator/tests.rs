use std::{
    ops::{Deref, DerefMut},
    sync::{Mutex, MutexGuard},
};

use serial_test::serial;

pub use super::*;

struct EmuGuard<'a>(Emulator, #[allow(dead_code)] MutexGuard<'a, ()>);

impl Deref for EmuGuard<'_> {
    type Target = Emulator;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EmuGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn run(asm: &str) -> (EmuGuard<'_>, u32) {
    static LOCK: Mutex<()> = Mutex::new(());

    // important. this'll keep us from allocating > 1 4gb alloc at a time
    let _guard = LOCK.lock().unwrap();

    #[rustfmt::skip]
    let asm = format!("
        {asm}
        hlt
    ");

    let data = graft::assemble(&asm).expect("assemblage to succeed");
    let mut emu = Emulator::new(&data).expect("emulator to be created");

    let code = emu.run().expect("run to succeed");

    (EmuGuard(emu, _guard), code)
}

#[test]
#[serial]
fn test() {
    let (emu, _) = run("mov t0, 5");
    assert_eq!(emu.cpu.gp.t0, 5);
}

#[test]
#[serial]
fn test2() {
    let (emu, _) = run("mov t0, 56");
    assert_eq!(emu.cpu.gp.t0, 5);
}
