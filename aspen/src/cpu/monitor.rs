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
    /// Change size / location
    Update { base: BitSize },
    /// Stop running
    Stop,
}

enum Command {
    /// Draw call finished
    Finished,
}

#[derive(Debug, Copy, Clone)]
pub struct MonitorArgs {
    width: u16,
    height: u16,
    fps: u16,
}

impl From<[u8; 6]> for MonitorArgs {
    fn from(value: [u8; 6]) -> Self {
        Self {
            width: u16::from_le_bytes([
                value.first().copied().unwrap_or_default(),
                value.get(1).copied().unwrap_or_default(),
            ]),
            height: u16::from_le_bytes([
                value.get(2).copied().unwrap_or_default(),
                value.get(3).copied().unwrap_or_default(),
            ]),
            fps: u16::from_le_bytes([
                value.get(4).copied().unwrap_or_default(),
                value.get(5).copied().unwrap_or_default(),
            ]),
        }
    }
}

#[derive(Debug)]
pub struct Monitor {
    tx: Sender<ReqCommand>,
    rx: Receiver<Command>,
}

impl Monitor {
    pub fn new(mut addr: BitSize, mmu: Arc<Mmu>) -> minifb::Result<Self> {
        let (tx, rx) = channel();
        let (reply_tx, reply_rx) = channel();
        let this = Self { tx, rx: reply_rx };

        thread::spawn(move || {
            let mut arg_buf = [0u8; size_of::<MonitorArgs>()];
            mmu.memcpy(addr, &mut arg_buf).unwrap();

            let mut args = MonitorArgs::from(arg_buf);

            let mut vram = vec![0u32; args.width as usize * args.height as usize];
            let mut vram_base = addr + size_of::<MonitorArgs>() as u32;

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
                        mmu.memcpy(vram_base, vram_slice).unwrap();

                        window
                            .update_with_buffer(&vram, args.width as usize, args.height as usize)
                            .unwrap();

                        reply_tx.send(Command::Finished).unwrap();
                    }

                    ReqCommand::Update { base } => {
                        addr = base;

                        let mut arg_buf = [0u8; size_of::<MonitorArgs>()];
                        mmu.memcpy(addr, &mut arg_buf).unwrap();
                        args = MonitorArgs::from(arg_buf);

                        window = Box::new(
                            Window::new("", args.width as _, args.height as _, opts).unwrap(),
                        );

                        window.set_target_fps(args.fps as _);

                        vram_base = addr + size_of::<MonitorArgs>() as u32;
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

    pub fn update(&self, base: BitSize) {
        self.tx.send(ReqCommand::Update { base }).unwrap();
    }

    pub fn stop(&self) {
        self.tx.send(ReqCommand::Stop).unwrap();
    }
}
