use cpal::{Device, Format};

use lazy_static::lazy_static;

lazy_static! {
static ref DEVICE: Device = cpal::default_input_device().expect("no default device");
static ref FORMAT: Format = DEVICE.default_input_format().expect("no default format");
}

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
    info!("Device: {}", default_device().name());
    info!("Format: {:?}", default_format());
}

#[inline]
fn default_device() -> &'static Device {
    &DEVICE
}

#[inline]
fn default_format() -> &'static Format {
    &FORMAT
}
