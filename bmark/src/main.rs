use graft_run::run;
use std::ptr;
use std::time::Instant;
use aspen::emulator::Emulator;

fn main() {
    let mut emu = Emulator::new(&[]).expect("creation to succeed");

    let mut run = |asm| {
        let asm = format!("{asm}\n\n; auto inserted\nhlt");

        let data = match graft::assemble("<input>.asm", &asm) {
            Ok(d) => d,
            Err(e) => panic!("{e}"),
        };

        emu.mem.zeroize().expect("zeroize to succeed");

        // prefault the pages to test the emulator's performance,
        // not the os's lazy alloc overhead
        //
        // TODO: for linux, use MAP_POPULATE
        for b in emu
            .mem.data_mut().iter_mut()
            .step_by(4096 /* min page size on modern oses */)
        {
            unsafe {
            ptr::write_volatile(b, 1);
            ptr::write_volatile(b, 0);
            }
        }

        emu.write_program(&data);

        let start = Instant::now();

        emu.run().expect("run to succeed");

        let elapsed = start.elapsed();

        println!(
            "instrs = {}, elapsed = {:?}, ns/instr = {:.3?}",
            emu.cpu.clk,
            elapsed,
            elapsed.as_nanos() as f64 / emu.cpu.clk as f64,
        );
    };

    let emu = run! {
        #addr 100
        helloworld:
            #d "Hello, world!\n"

        helloworldLen = $ - helloworld

        #addr 0
        _start:
        mov t0, 5
        mov t2, 0
        sl t5, t2, t0

        mov a0, 3
        mov a1, 7
        push t0
        call calculate
        pop t0

        mov t5, helloworld
        mov t6, helloworldLen + 100
        ;pr t5, t6

        jnez a0, exit
        jmp exit+8 ; nop for days

        calculate:
        sl a0, a0, a1
        ret

        exit:
        hlt
    };
}
