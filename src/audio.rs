use std::cmp::{max, min};

use cpal::{Device, Format};
use failure::Error;
use failure::format_err;
use futures::Stream;
use futures::sync::mpsc::UnboundedSender;
use hound::Sample;

use lazy_static::lazy_static;

lazy_static! {
static ref DEVICE: Device = cpal::default_input_device().expect("no default device");
static ref FORMAT: Format = DEVICE.default_input_format().expect("no default format");
}

#[inline]
pub fn sample_rate() -> u32 {
    format().sample_rate.0
}

#[inline]
pub fn bit_depth() -> usize {
    format().data_type.sample_size() * 8
}

#[inline]
pub fn channels() -> u16 {
    format().channels
}

pub fn print_device_info() {
    info!("Device: {}", device().name());
    info!("Format: {:?}", format());
}

pub fn start() -> impl Stream<Item=Vec<u8>, Error=Error> {
    let event_loop = cpal::EventLoop::new();
    let stream_id = event_loop.build_input_stream(device(), format())
        .expect("Failed to build input stream");
    event_loop.play_stream(stream_id);
    let (tx, rx) = futures::sync::mpsc::unbounded();
    std::thread::spawn(move || {
        info!("EventLoop thread started");
        event_loop.run(|stream_id, data| {
            let mut vec: Vec<u8>;
            // Otherwise write to the wav writer.
            match data {
                cpal::StreamData::Input { buffer: cpal::UnknownTypeInputBuffer::U16(buffer) } => {
                    vec = Vec::with_capacity(buffer.len() * 2);
                    for sample in buffer.iter() {
                        let sample = cpal::Sample::to_i16(sample);
                        sample.write(&mut vec, 16).expect("failed to write sample");
                    }
                }
                cpal::StreamData::Input { buffer: cpal::UnknownTypeInputBuffer::I16(buffer) } => {
                    vec = Vec::with_capacity(buffer.len() * 2);
                    for &sample in buffer.iter() {
                        sample.write(&mut vec, 16).expect("failed to write sample");
                    }
                }
                cpal::StreamData::Input { buffer: cpal::UnknownTypeInputBuffer::F32(buffer) } => {
                    vec = Vec::with_capacity(buffer.len() * 4);
                    for &sample in buffer.iter() {
                        let mut i64_sample = (sample as f64 * 2147483648.0f64).round() as i64;
                        i64_sample = max(i64_sample, -2147483648);
                        i64_sample = min(i64_sample, 2147483647);
                        let int_sample = i64_sample as i32;
                        int_sample.write(&mut vec, 32).expect("failed to write sample");
                    }
                }
                _ => {
                    vec = Vec::new(); // empty, no memory allocated
                }
            }
            if !send(vec, &tx, 4096) {
                info!("Session stopped");
                event_loop.destroy_stream(stream_id);
            }
        });
        info!("EventLoop thread quit");
    });
    rx.map_err(|_| format_err!("Error"))
}

#[inline]
fn send(buff: Vec<u8>, tx: &UnboundedSender<Vec<u8>>, chunk_size: usize) -> bool {
    if buff.len() <= chunk_size {
        return send0(buff, tx);
    } else {
        let mut p: usize = 0;
        while p < buff.len() {
            let chunk = &buff[p..min(p + chunk_size, buff.len())];
            if !send0(chunk.to_owned(), tx) {
                return false;
            }
            p += chunk.len();
        }
        return true;
    }
}

#[inline]
fn send0(buff: Vec<u8>, tx: &UnboundedSender<Vec<u8>>) -> bool {
    let size = buff.len();
    let successful = tx.unbounded_send(buff).is_ok();
    if successful {
        debug!("transferred {} bytes", size);
    }
    return successful;
}

#[inline]
fn device() -> &'static Device {
    &DEVICE
}

#[inline]
fn format() -> &'static Format {
    &FORMAT
}
