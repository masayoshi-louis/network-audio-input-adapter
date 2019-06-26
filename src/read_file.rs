use std::cmp::{max, min};

use failure::Error;
use failure::format_err;
use futures::Stream;
use futures::sync::mpsc::UnboundedSender;
use hound::Sample;

pub fn start() -> impl Stream<Item=Vec<u8>, Error=Error> {
    let mut reader = hound::WavReader::open("./recorded.wav").unwrap();
    let (tx, rx) = futures::sync::mpsc::unbounded();
    std::thread::spawn(move || {
        let mut chunk_buff = new_chunk();
        for sample in reader.samples::<f32>() {
            match sample {
                Ok(sample) => {
                    let mut int_sample = (sample * 8388608.0f32).round() as i32;
                    int_sample = max(int_sample, -8388608);
                    int_sample = min(int_sample, 8388607);
                    int_sample.write(&mut chunk_buff, 24).expect("failed to write sample");
                    send_if_full(&mut chunk_buff, &tx);
                }
                Err(_) => {
                    panic!("failure");
                }
            }
        }
        info!("EOF");
    });
    rx.map_err(|_| format_err!("Error"))
}

#[inline]
fn new_chunk() -> Vec<u8> {
    Vec::with_capacity(3 * 48000 / 10 * 2)
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
