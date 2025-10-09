use std::{
    ops::{Deref, DerefMut},
    sync::{LazyLock, Mutex, MutexGuard},
};

use graft_run::run;
use serial_test::serial;

pub use super::*;

struct EmuGuard<'a>(MutexGuard<'a, Emulator>);

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

impl Drop for EmuGuard<'_> {
    fn drop(&mut self) {
        self.cpu.zeroize();
        self.mem.zeroize().expect("zeroize to succeed");
    }
}

fn run(asm: &str) -> EmuGuard<'_> {
    static LOCK: LazyLock<Mutex<Emulator>> =
        LazyLock::new(|| Mutex::new(Emulator::new(&[]).expect("creation to succeed")));

    LOCK.clear_poison();

    // important. this will keep it synchronous
    let mut guard = LOCK.lock().unwrap();

    let asm = format!("{asm}\n\n; auto inserted\nhlt");

    let data = match graft::assemble("<input>.asm", &asm) {
        Ok(d) => d,
        Err(e) => panic!("{e}"),
    };

    guard.write_program(&data);

    guard.run().expect("run to succeed");

    EmuGuard(guard)
}

#[test]
#[serial]
fn test_hlt() {
    let emu = run! { hlt };
    assert_eq!(emu.cpu.pc, 2);
}

#[test]
#[serial]
fn test2() {
    let emu = run! { mov t0, 56 };
    assert_eq!(emu.cpu.gp.t0, 56);
}
