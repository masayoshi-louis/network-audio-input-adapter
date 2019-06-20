use futures::future;
use futures::future::FutureResult;
use futures::prelude::*;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::service_fn;

fn main() {
    let addr = ([0, 0, 0, 0], 3000).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn(handle))
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", addr);
    hyper::rt::run(server);
}

fn handle(req: Request<Body>) -> FutureResult<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::HEAD, "/stream.raw") => {
            println!("get head request");
            set_headers(&mut response);
        }
        (&Method::GET, "/stream.raw") => {
            println!("get stream request");
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
    response.headers_mut().insert("Content-Type", "application/x-hqplayer-raw".parse().unwrap());
    response.headers_mut().insert("X-HQPlayer-Raw-Title", "Roon".parse().unwrap());
    response.headers_mut().insert("X-HQPlayer-Raw-SampleRate", "44100".parse().unwrap());
    response.headers_mut().insert("X-HQPlayer-Raw-Channels", "2".parse().unwrap());
    response.headers_mut().insert("X-HQPlayer-Raw-Format", "int16le".parse().unwrap());
}
