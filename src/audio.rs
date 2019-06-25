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
    min(24, format().data_type.sample_size() * 8)
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
        let mut active = true;
        let mut stopped = false;
        let mut chunk_buff = new_chunk();
        info!("EventLoop thread started");
        event_loop.run(|stream_id, data| {
            if !active {
                if !stopped {
                    info!("Session stopped");
                    event_loop.destroy_stream(stream_id);
                    stopped = true;
                }
                return;
            }

            match data {
                cpal::StreamData::Input { buffer: cpal::UnknownTypeInputBuffer::U16(buffer) } => {
                    for sample in buffer.iter() {
                        let sample = cpal::Sample::to_i16(sample);
                        sample.write(&mut chunk_buff, 16).expect("failed to write sample");
                        active = send_if_full(&mut chunk_buff, &tx);
                    }
                }
                cpal::StreamData::Input { buffer: cpal::UnknownTypeInputBuffer::I16(buffer) } => {
                    for &sample in buffer.iter() {
                        sample.write(&mut chunk_buff, 16).expect("failed to write sample");
                        active = send_if_full(&mut chunk_buff, &tx);
                    }
                }
                cpal::StreamData::Input { buffer: cpal::UnknownTypeInputBuffer::F32(buffer) } => {
                    for &sample in buffer.iter() {
                        let mut int_sample = (sample * 8388608.0f32).round() as i32;
                        int_sample = max(int_sample, -8388608);
                        int_sample = min(int_sample, 8388607);
                        int_sample.write(&mut chunk_buff, 24).expect("failed to write sample");
                        active = send_if_full(&mut chunk_buff, &tx);
                    }
                }
                _ => {}
            }
        });
        info!("EventLoop thread quit");
    });
    rx.map_err(|_| format_err!("Error"))
}

#[inline]
fn new_chunk() -> Vec<u8> {
    // 100ms buffer
    Vec::with_capacity(bit_depth() / 8 * sample_rate() as usize / 10 * channels() as usize)
}

#[inline]
fn send_if_full(buff: &mut Vec<u8>, tx: &UnboundedSender<Vec<u8>>) -> bool {
    if buff.len() == buff.capacity() {
        let buff = std::mem::replace(buff, new_chunk());
        send(buff, tx)
    } else {
        true
    }
}

#[inline]
fn send(buff: Vec<u8>, tx: &UnboundedSender<Vec<u8>>) -> bool {
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
