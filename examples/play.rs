#[macro_use]
extern crate log;

use std::env;

use cpal::{Format, OutputBuffer, Sample as CpalSample, SampleFormat, SampleRate};
use futures::stream::Wait as StreamWait;
use futures::sync::mpsc;
use futures::sync::mpsc::UnboundedReceiver;
use hound::Sample;
use hyper::Client;
use hyper::rt::{self, Future, Stream};

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    let url = match env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("Usage: play <url>");
            return;
        }
    };

    let device_name = env::args().nth(2);

    // HTTPS requires picking a TLS implementation, so give a better
    // warning if the user tries to request an 'https' URL.
    let url = url.parse::<hyper::Uri>().unwrap();
    if url.scheme_part().map(|s| s.as_ref()) != Some("http") {
        println!("This example only works with 'http' URLs.");
        return;
    }

    let (tx, rx) = mpsc::unbounded();

    play_it(rx, device_name.as_ref());

    // Run the runtime with the future trying to fetch and print this URL.
    //
    // Note that in more complicated use cases, the runtime should probably
    // run on its own, and futures should just be spawned into it.
    rt::run(fetch_url(url, tx));
}

fn fetch_url(url: hyper::Uri, tx: mpsc::UnboundedSender<u8>) -> impl Future<Item=(), Error=()> {
    let client = Client::new();

    client
        // Fetch the URL...
        .get(url)
        // And then, if we get a response back...
        .and_then(|res| {
            info!("Response: {}", res.status());
            info!("Headers: {:#?}", res.headers());

            // The body is a stream, and for_each returns a new Future
            // when the stream is finished, and calls the closure on
            // each chunk of the body...
            res.into_body().for_each(move |chunk| {
                debug!("Chunk size {}", chunk.len());
                for byte in chunk {
                    tx.unbounded_send(byte).expect("channel failure");
                }
                futures::future::ok(())
            })
        })
        // If all good, just tell the user...
        .map(|_| {
            println!("\n\nDone.");
        })
        // If there was an error, let the user know...
        .map_err(|err| {
            eprintln!("Error {}", err);
        })
}

fn play_it(rx: mpsc::UnboundedReceiver<u8>, device_name: Option<impl AsRef<str>>) {
    let mut devices = cpal::devices();
    let device = if let Some(name) = device_name {
        devices.find(|x| {
            x.name() == name.as_ref() && x.supported_output_formats().expect("can not get output formats").peekable().peek().is_some()
        }).expect("can not find device")
    } else {
        cpal::default_output_device().expect("can not find default device")
    };

    println!("Using Device {}", device.name());
    let mut output_formats = device.supported_output_formats().expect("can not get output formats").peekable();
    if output_formats.peek().is_some() {
        println!("All supported output stream formats:");
        for (format_index, format) in output_formats.enumerate() {
            println!("  {}. {:?}", format_index + 1, format);
        }
    }

    let format = Format {
        channels: 2,
        sample_rate: SampleRate(48000),
        data_type: SampleFormat::F32,
    };
    let event_loop = cpal::EventLoop::new();
    let stream_id = event_loop.build_output_stream(&device, &format).expect("can not create output stream");
    event_loop.play_stream(stream_id.clone());

    std::thread::spawn(move || {
        info!("Started!");
        let mut source = rx.wait();
        let mut sample_buff = [0u8; 4];
        event_loop.run(|_, data| {
            match data {
                cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::U16(_buffer) } => {
                    panic!("unsupported");
                }
                cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::I16(buffer) } => {
                    transfer(buffer, &mut source, &mut sample_buff, 16);
                }
                cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer) } => {
                    //transfer(buffer, &mut source, &mut sample_buff, 32);
                    for out_sample in buffer.iter_mut() {
                        for i in 0..3 {
                            sample_buff[i] = source.next().expect("can not get sample from channel").expect("channel error");
                        }
                        let mut sample_buff = &sample_buff[..];
                        let sample = i32::read(
                            &mut sample_buff,
                            hound::SampleFormat::Int,
                            3,
                            24,
                        ).expect("can not read sample");
                        *out_sample = ((sample as f64) / 8388608.0f64) as f32;
                    }
                }
                _ => (),
            }
        });
    });
}

#[inline]
fn transfer<S>(mut buffer: OutputBuffer<S>,
               source: &mut StreamWait<UnboundedReceiver<u8>>,
               sample_buff: &mut [u8],
               bits: u16)
    where S: CpalSample, S: Sample {
    let bytes = bits / 8;
    for out_sample in buffer.iter_mut() {
        for i in 0..(bytes as usize) {
            sample_buff[i] = source.next().expect("can not get sample from channel").expect("channel error");
        }
        let mut sample_buff = &sample_buff[..];
        *out_sample = S::read(
            &mut sample_buff,
            hound::SampleFormat::Float,
            bits / 8,
            bits,
        ).expect("can not read sample");
    }
}
