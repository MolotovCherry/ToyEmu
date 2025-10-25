use std::{
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use minifb::{Scale, ScaleMode, Window, WindowOptions};

use crate::{BitSize, memory::Memory};

enum ReqCommand {
    /// Request redraw
    Draw { mem: Memory },
    /// Change size / location
    Update { base: BitSize, args: MonitorArgs },
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

impl From<&[u8]> for MonitorArgs {
    fn from(value: &[u8]) -> Self {
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
    pub fn new(mut args: MonitorArgs, mut base: BitSize) -> minifb::Result<Self> {
        let (tx, rx) = channel();
        let (reply_tx, reply_rx) = channel();
        let this = Self { tx, rx: reply_rx };

        thread::spawn(move || {
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

            let mut size =
                size_of::<u32>() as BitSize * args.width as BitSize * args.height as BitSize;

            loop {
                let c = match rx.recv() {
                    Ok(c) => c,
                    Err(_) => break,
                };

                match c {
                    ReqCommand::Draw { mem } => {
                        let addr = base;
                        let mem = &mem[addr..addr + size];

                        let mem = bytemuck::cast_slice(mem);

                        window
                            .update_with_buffer(mem, args.width as usize, args.height as usize)
                            .unwrap();

                        reply_tx.send(Command::Finished).unwrap();
                    }

                    ReqCommand::Update {
                        base: _base,
                        args: _args,
                    } => {
                        base = _base;
                        args = _args;
                        size = size_of::<u32>() as BitSize
                            * args.width as BitSize
                            * args.height as BitSize;
                    }

                    ReqCommand::Stop => break,
                }
            }
        });

        Ok(this)
    }

    pub fn draw(&self, mem: Memory) {
        self.tx.send(ReqCommand::Draw { mem }).unwrap();
        self.rx.recv().unwrap();
    }

    pub fn update(&self, base: BitSize, args: MonitorArgs) {
        self.tx.send(ReqCommand::Update { base, args }).unwrap();
    }

    pub fn stop(&self) {
        self.tx.send(ReqCommand::Stop).unwrap();
    }
}
