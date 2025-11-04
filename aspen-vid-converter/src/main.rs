use std::env;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::Path;

use ffmpeg::format::{Pixel, input};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use ffmpeg_next as ffmpeg;
use image::{DynamicImage, RgbImage};

fn main() -> Result<(), ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let path = &env::args().nth(1).expect("Cannot open file.");

    let out_file = Path::new(path)
        .file_stem()
        .expect("please give your source file a filename");

    let p = Path::new(out_file).with_extension("bin");
    let mut out_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(p)
        .expect("failed to create file");

    let mut ictx = input(path)?;
    let input = ictx
        .streams()
        .best(Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)?;
    let video_stream_index = input.index();

    let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;

    let mut scaler = Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )?;

    let mut frame_index = 0;

    let mut receive_and_process_decoded_frames =
        |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
            let mut decoded = Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut rgb_frame = Video::empty();
                scaler.run(&decoded, &mut rgb_frame)?;
                convert(&rgb_frame, frame_index, &mut out_file).unwrap();
                frame_index += 1;
            }
            Ok(())
        };

    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;
            receive_and_process_decoded_frames(&mut decoder)?;
        }
    }
    decoder.send_eof()?;
    receive_and_process_decoded_frames(&mut decoder)?;

    Ok(())
}

fn convert(
    frame: &Video,
    _index: usize,
    out_file: &mut File,
) -> std::result::Result<(), std::io::Error> {
    let data = frame.data(0);

    let width = frame.width();
    let height = frame.height();

    let image = RgbImage::from_raw(width, height, data.into()).unwrap();
    let image = DynamicImage::ImageRgb8(image);
    let image = image.grayscale();
    let image = image.as_luma8().unwrap();

    let mut data = Vec::new();

    let mut last = 0;

    let mut start = true;
    let mut c = 0u32;
    for p in image.pixels() {
        let pixel = p.0[0];

        if start {
            last = pixel;
            start = false;
        }

        if pixel == last {
            c += 1;
        } else {
            data.extend(c.to_le_bytes());
            data.push(last);
            last = pixel;
            c = 1;
        }
    }

    data.extend(c.to_le_bytes());
    data.push(last);

    data.extend([0xff, 0xff, 0xff, 0xff]);

    out_file.write_all(&data)?;

    Ok(())
}
