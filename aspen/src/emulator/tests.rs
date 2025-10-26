mod emu;

use enumflags2::BitFlag as _;
use serial_test::serial;

pub use super::*;
use emu::macros::*;

#[test]
#[serial]
fn test_prot() {
    let handle = |emu: &mut Emulator| {
        let mem = emu.mem.get_mut().unwrap();
        mem.change_prot(0..100, Prot::Read | Prot::Write).unwrap();
    };

    let res = try_run_with! {
        handle,

        mov t0, 0x0 ; location
        str.b [t0], 0x00
    };

    // test execution can't execute
    let res = res.map(|_| ());
    let e = Err(EmuError::Mem(MemError::PageFault(Prot::Execute.into())));
    assert_eq!(e, res);

    // --

    let handle = |emu: &mut Emulator| {
        let mem = emu.mem.get_mut().unwrap();
        mem.change_prot(0x12345678, Prot::empty()).unwrap();
    };

    let res = try_run_with! {
        handle,

        mov t0, 0x12345678 ; location
        str.b [t0], 0x00
    };

    // test execution can't execute
    let res = res.map(|_| ());
    let e = Err(EmuError::Cpu(CpuError::Mem(MemError::PageFault(
        Prot::Write.into(),
    ))));
    assert_eq!(e, res);
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
         mov t0, 0x12345678
         mov t1, t0
    };

    assert_eq!(emu.cpu.gp.t0, 0x12345678);
    assert_eq!(emu.cpu.gp.t1, 0x12345678);
}

#[test]
#[serial]
fn test_rdclk() {
    let emu = run! {
        mov zr, zr   ; 1 cycle
        mov zr, zr   ; 1 cycle
        push t0      ; 2 cycles
        pop t0       ; 2 cycles
        rdclk t0, t1 ; 6!
    };

    let val = (emu.cpu.gp.t1 as u64) << 32 | emu.cpu.gp.t0 as u64;

    assert_eq!(val, 6);
}

#[test]
#[serial]
fn test_tme() {
    let before = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let emu = run! {
        tme t0, t1, t2, t3
    };

    let after = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    #[rustfmt::skip]
    let time: u128 =
        (emu.cpu.gp.t3 as u128) << 96 |
        (emu.cpu.gp.t2 as u128) << 64 |
        (emu.cpu.gp.t1 as u128) << 32 |
        (emu.cpu.gp.t0 as u128);

    assert!(time > before);
    assert!(time < after);
}

#[test]
#[serial]
fn test_ld() {
    let emu = run! {
        mov t0, 0x12345678
        str [t0], 0x52355678
        ld t1, [t0]
    };

    assert_eq!(emu.cpu.gp.t1, 0x52355678);
}

#[test]
#[serial]
fn test_ldw() {
    let emu = run! {
        mov t0, 0x12345678
        str.w [t0], 0x5678
        ld.w t1, [t0]
    };

    assert_eq!(emu.cpu.gp.t1, 0x5678);
}

#[test]
#[serial]
fn test_ldb() {
    let emu = run! {
        mov t0, 0xFFFFFFFF
        str.b [t0], 0x78
        ld.b t1, [t0]
    };

    assert_eq!(emu.cpu.gp.t1, 0x78);
}

#[test]
#[serial]
fn test_str() {
    let emu = run! {
        mov t0, 0x12345678 ; location
        str [t0], 0x12345678

        mov t0, 0x11223344 ; location
        mov t1, 0x11223344 ; val
        str [t0], t1
    };

    let a = 0x12345678;
    let data: [u8; 4] = emu.mem[a..a + 4].try_into().unwrap();
    let val = u32::from_le_bytes(data);
    assert_eq!(val, a);

    let b = 0x11223344;
    let data: [u8; 4] = emu.mem[b..b + 4].try_into().unwrap();
    let val = u32::from_le_bytes(data);
    assert_eq!(val, b);
}

#[test]
#[serial]
fn test_strw() {
    let emu = run! {
        mov t0, 0x1234 ; location
        str.w [t0], 0x00001234

        mov t0, 0x1122 ; location
        mov t1, 0x00001122 ; val
        str.w [t0], t1
    };

    let a = 0x00001234;
    let data: [u8; 2] = emu.mem[a..a + 2].try_into().unwrap();
    let val = u16::from_le_bytes(data);
    assert_eq!(val as u32, a);

    let b = 0x00001122;
    let data: [u8; 2] = emu.mem[b..b + 2].try_into().unwrap();
    let val = u16::from_le_bytes(data);
    assert_eq!(val as u32, b);
}

#[test]
#[serial]
fn test_strb() {
    let emu = run! {
        mov t0, 0x1000 ; location
        str.b [t0], 0x12

        mov t0, 0xFFFFFFFF ; location
        mov t1, 0x13 ; val
        str.b [t0], t1
    };

    assert_eq!(emu.mem[0x1000], 0x12);
    assert_eq!(emu.mem[0xFFFFFFFF], 0x13);
}
