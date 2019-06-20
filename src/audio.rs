use std::env;

use cpal::{Device, Format, SampleFormat};

#[inline]
pub fn sample_rate() -> u32 {
    default_format().sample_rate.0
}

#[inline]
pub fn bit_depth() -> usize {
    default_format().data_type.sample_size() * 8
}

#[inline]
pub fn channels() -> u16 {
    default_format().channels
}

pub fn print_device_info() {
    let device = default_device();
    info!("Device: {}", device.name());
    info!("Format: {:?}", device.default_input_format().expect("no default format"));
}

#[inline]
fn default_device() -> Device {
    cpal::default_input_device().expect("no default device")
}

#[inline]
fn default_format() -> Format {
    default_device().default_input_format().expect("no default format")
}
