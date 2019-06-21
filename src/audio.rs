use cpal::{Device, Format};
use failure::Error;
use failure::format_err;
use futures::Stream;
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
                        sample.write(&mut vec, 32).expect("failed to write sample");
                    }
                }
                _ => {
                    vec = Vec::new(); // empty, no memory allocated
                }
            }
            if tx.unbounded_send(vec).is_err() {
                info!("Session stopped");
                event_loop.destroy_stream(stream_id);
            }
        });
        info!("EventLoop thread quit");
    });
    rx.map_err(|_| format_err!("Error"))
}

#[inline]
fn device() -> &'static Device {
    &DEVICE
}

#[inline]
fn format() -> &'static Format {
    &FORMAT
}
