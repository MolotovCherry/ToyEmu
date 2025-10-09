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
fn test_registers() {
    let emu = run! {
         mov zr, 42 ; non zero write
         mov ra, 1
         mov sp, 2
         mov gp, 3
         mov tp, 4
         mov t0, 5
         mov t1, 6
         mov t2, 7
         mov t3, 8
         mov t4, 9
         mov t5, 10
         mov t6, 11
         mov s0, 12
         mov s1, 13
         mov s2, 14
         mov s3, 15
         mov s4, 16
         mov s5, 17
         mov s6, 18
         mov s7, 19
         mov s8, 20
         mov s9, 21
         mov s10, 22
         mov s11, 23
         mov a0, 24
         mov a1, 25
         mov a2, 26
         mov a3, 27
         mov a4, 28
         mov a5, 29
         mov a6, 30
         mov a7, 31
    };

    assert_eq!(emu.cpu.gp.zr, 0); // important to be 0
    assert_eq!(emu.cpu.gp.ra, 1);
    assert_eq!(emu.cpu.gp.sp, 2);
    assert_eq!(emu.cpu.gp.gp, 3);
    assert_eq!(emu.cpu.gp.tp, 4);
    assert_eq!(emu.cpu.gp.t0, 5);
    assert_eq!(emu.cpu.gp.t1, 6);
    assert_eq!(emu.cpu.gp.t2, 7);
    assert_eq!(emu.cpu.gp.t3, 8);
    assert_eq!(emu.cpu.gp.t4, 9);
    assert_eq!(emu.cpu.gp.t5, 10);
    assert_eq!(emu.cpu.gp.t6, 11);
    assert_eq!(emu.cpu.gp.s0, 12);
    assert_eq!(emu.cpu.gp.s1, 13);
    assert_eq!(emu.cpu.gp.s2, 14);
    assert_eq!(emu.cpu.gp.s3, 15);
    assert_eq!(emu.cpu.gp.s4, 16);
    assert_eq!(emu.cpu.gp.s5, 17);
    assert_eq!(emu.cpu.gp.s6, 18);
    assert_eq!(emu.cpu.gp.s7, 19);
    assert_eq!(emu.cpu.gp.s8, 20);
    assert_eq!(emu.cpu.gp.s9, 21);
    assert_eq!(emu.cpu.gp.s10, 22);
    assert_eq!(emu.cpu.gp.s11, 23);
    assert_eq!(emu.cpu.gp.a0, 24);
    assert_eq!(emu.cpu.gp.a1, 25);
    assert_eq!(emu.cpu.gp.a2, 26);
    assert_eq!(emu.cpu.gp.a3, 27);
    assert_eq!(emu.cpu.gp.a4, 28);
    assert_eq!(emu.cpu.gp.a5, 29);
    assert_eq!(emu.cpu.gp.a6, 30);
    assert_eq!(emu.cpu.gp.a7, 31);
}

#[test]
#[serial]
fn test_mov() {
    let emu = run! {
         mov t0, 56
         mov t1, t0
    };
    assert_eq!(emu.cpu.gp.t0, 56);
    assert_eq!(emu.cpu.gp.t1, 56);
}

#[test]
#[serial]
fn test_rdclk() {
    let emu = run! {
        rdclk t0, t1
    };

    assert_eq!(emu.cpu.gp.t0, 0);
    assert_eq!(emu.cpu.gp.t1, 2);
}
