use std::{
    sync::{
        Arc,
        mpsc::{Receiver, Sender, channel},
    },
    thread,
};

use minifb::{Scale, ScaleMode, Window, WindowOptions};

use crate::{BitSize, mmu::Mmu};

enum ReqCommand {
    /// Request redraw
    Draw,
    /// Stop running
    Stop,
}

enum Command {
    /// Draw call finished
    Finished,
}

#[derive(Debug, Copy, Clone)]
pub struct MonitorArgs {
    pub width: u16,
    pub height: u16,
    pub fps: u16,
}

#[derive(Debug)]
pub struct Monitor {
    tx: Sender<ReqCommand>,
    rx: Receiver<Command>,
}

impl Monitor {
    pub fn new(addr: BitSize, mmu: Arc<Mmu>, args: MonitorArgs) -> minifb::Result<Self> {
        let (tx, rx) = channel();
        let (reply_tx, reply_rx) = channel();
        let this = Self { tx, rx: reply_rx };

        thread::spawn(move || {
            let mut vram = vec![0u32; args.width as usize * args.height as usize];

            let opts = WindowOptions {
                borderless: false,
                title: true,
                resize: true,
                scale: Scale::FitScreen,
                scale_mode: ScaleMode::AspectRatioStretch,
                topmost: false,
                transparency: false,
                none: false,
            };

            #[rustfmt::skip]
            let mut window = Box::new(Window::new(
                "",
                args.width as _,
                args.height as _,
                opts
            ).unwrap());

            window.set_target_fps(args.fps as _);

            while let Ok(c) = rx.recv() {
                match c {
                    ReqCommand::Draw => {
                        let vram_slice = bytemuck::must_cast_slice_mut::<_, u8>(&mut vram);
                        mmu.memcpy(addr, vram_slice).unwrap();

                        window
                            .update_with_buffer(&vram, args.width as usize, args.height as usize)
                            .unwrap();

                        reply_tx.send(Command::Finished).unwrap();
                    }

                    ReqCommand::Stop => break,
                }
            }
        });

        Ok(this)
    }

    pub fn draw(&self) {
        self.tx.send(ReqCommand::Draw).unwrap();
        self.rx.recv().unwrap();
    }

    pub fn stop(&self) {
        self.tx.send(ReqCommand::Stop).unwrap();
    }
}
