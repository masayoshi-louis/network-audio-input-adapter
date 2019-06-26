#[macro_use]
extern crate log;

use std::env;

use futures::future;
use futures::future::FutureResult;
use futures::prelude::*;
use http::header::HdrName;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::service_fn;

mod audio;
mod read_file;

const HTTP_STREAM_PATH: &'static str = "/stream.raw";

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info,network_audio_input_adapter=trace");
    }
    env_logger::init();

    audio::print_device_info();

    let addr = ([0, 0, 0, 0], 3000).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn(handle))
        .map_err(|e| error!("server error: {}", e));

    info!("Listening on http://{}", addr);
    hyper::rt::run(server);
}

fn handle(req: Request<Body>) -> FutureResult<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());
    let req_tuple = (req.method(), req.uri().path());
    {
        let (m, p) = req_tuple;
        debug!("Request: {} {}", m, p);
    }

    match req_tuple {
        (&Method::HEAD, HTTP_STREAM_PATH) => {
            set_headers(&mut response);
        }
        (&Method::GET, HTTP_STREAM_PATH) => {
            set_headers(&mut response);
            *response.body_mut() = Body::wrap_stream(audio::start());
        }
        (_, HTTP_STREAM_PATH) => {
            *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
        }
        (&Method::HEAD, "/file.raw") => {
            set_headers(&mut response);
        }
        (&Method::GET, "/file.raw") => {
            set_headers(&mut response);
            *response.body_mut() = Body::wrap_stream(read_file::start());
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    }

    return future::ok(response);
}

fn set_headers(response: &mut Response<Body>) {
    response.headers_mut().insert2(HdrName::custom(b"Content-Type", true), "application/x-hqplayer-raw".parse().unwrap());
    response.headers_mut().insert2(HdrName::custom(b"X-HQPlayer-Raw-Title", true), "Network Input".parse().unwrap());
    response.headers_mut().insert2(HdrName::custom(b"X-HQPlayer-Raw-SampleRate", true), audio::sample_rate().to_string().parse().unwrap());
    response.headers_mut().insert2(HdrName::custom(b"X-HQPlayer-Raw-Channels", true), audio::channels().to_string().parse().unwrap());
    response.headers_mut().insert2(HdrName::custom(b"X-HQPlayer-Raw-Format", true), format!("int{}le", audio::bit_depth()).parse().unwrap());
}
