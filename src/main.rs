#[macro_use]
extern crate log;

use std::env;

use futures::future;
use futures::future::FutureResult;
use futures::prelude::*;
use http::header::HdrName;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::service_fn;

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info,network_audio_input_adapter=trace");
    }
    env_logger::init();

    let addr = ([0, 0, 0, 0], 3000).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn(handle))
        .map_err(|e| eprintln!("server error: {}", e));

    info!("Listening on http://{}", addr);
    hyper::rt::run(server);
}

fn handle(req: Request<Body>) -> FutureResult<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::HEAD, "/stream.raw") => {
            trace!("HEAD request received");
            set_headers(&mut response);
        }
        (&Method::GET, "/stream.raw") => {
            trace!("GET request received");
            set_headers(&mut response);
            let (tx, body) = Body::channel();
        }
        (_, "/stream.raw") => {
            *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    }

    return future::ok(response);
}

fn set_headers(response: &mut Response<Body>) {
    response.headers_mut().insert2(HdrName::custom(b"Content-Type", true), "application/x-hqplayer-raw".parse().unwrap());
    response.headers_mut().insert2(HdrName::custom(b"X-HQPlayer-Raw-Title", true), "NetworkInput".parse().unwrap());
    response.headers_mut().insert2(HdrName::custom(b"X-HQPlayer-Raw-SampleRate", true), "44100".parse().unwrap());
    response.headers_mut().insert2(HdrName::custom(b"X-HQPlayer-Raw-Channels", true), "2".parse().unwrap());
    response.headers_mut().insert2(HdrName::custom(b"X-HQPlayer-Raw-Format", true), "int16le".parse().unwrap());
}
