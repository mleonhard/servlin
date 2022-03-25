# beatrice

[![crates.io version](https://img.shields.io/crates/v/beatrice.svg)](https://crates.io/crates/beatrice)
[![license: Apache 2.0](https://raw.githubusercontent.com/mleonhard/beatrice-rs/main/license-apache-2.0.svg)](http://www.apache.org/licenses/LICENSE-2.0)
[![unsafe forbidden](https://raw.githubusercontent.com/mleonhard/beatrice-rs/main/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![pipeline status](https://github.com/mleonhard/beatrice-rs/workflows/CI/badge.svg)](https://github.com/mleonhard/beatrice-rs/actions)

A modular HTTP server library in Rust.

## Features
- `forbid(unsafe_code)`
- Threaded request handlers:<br>
  `FnOnce(Request) -> Response + 'static + Clone + Send + Sync`
- Uses async code for excellent performance under load
- JSON
- Saves large request bodies to temp files
- Sends 100-Continue
- Limits number of threads and connections
- Modular: use pieces of the library, make async handlers, roll your own logging, etc.
- No macros or complicated type params
- Good test coverage (??%) - TODO: Update.

## Limitations
- New, not proven in production.
- Does not yet support:
  - request timeouts
  - chunked transfer-encoding for streaming uploads
  - gzip
  - TLS
  - automatically getting TLS certs via ACME
  - Denial-of-Service mitigation: source throttling, minimum throughput
  - Incomplete functional test suite
  - Missing load tests

## Examples
Complete example: [`examples/demo.rs`](examples/demo.rs).

Simple example:
```rust
use beatrice::{
    print_log_response,
    run_http_server,
    socket_addr_127_0_0_1,
    Request,
    Response
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

struct State {}

fn hello(_state: Arc<State>, req: Request) -> Result<Response, Response> {
    #[derive(Deserialize)]
    struct Input {
        name: String,
    }
    let input: Input = req.json()?;
    Ok(Response::json(
        200,
        json!({
            "message": format!("Hello, {}!", input.name)
        }),
    )
    .unwrap())
}

fn handle_req(state: Arc<State>, req: Request) -> Result<Response, Response> {
    match (req.method(), req.url().path(), req.content_type()) {
        ("GET", "/ping", _) => Ok(Response::text(200, "ok")),
        ("POST", "/hello", _) => hello(state, req),
        _ => Ok(Response::text(404, "Not found")),
    }
}

let state = Arc::new(State {});
let request_handler = move |req: Request| {
    print_log_response(
        req.method().to_string(),
        req.url().clone(),
        handle_req(state, req),
    )
};
let listen_addr = socket_addr_127_0_0_1(8000);
let num_threads = 10;
let max_conns = 1000;
let max_vec_body_len = 64 * 1024;
run_http_server(
    listen_addr,
    num_threads,
    max_conns,
    max_vec_body_len,
    request_handler,
)
.unwrap();
```

## Alternatives
- [`tide`](https://crates.io/crates/tide)
  - Popular
  - Does not support uploads (100-Continue): <https://github.com/http-rs/tide/issues/878>
- [`actix-web`](https://crates.io/crates/actix)
  - Very popular
  - Macros
  - Contains generous amounts of `unsafe` code
- [`rocket`](https://crates.io/crates/rocket)
  - Popular
  - Macros
  - Contains generous amounts of `unsafe` code
- [`rouille`](https://crates.io/crates/rouille)
  - Popular
  - Blocking handlers
  - [Uses an unbounded threadpool](https://github.com/tiny-http/tiny-http/issues/221)
    and [stops serving after failing once to spawn a thread](https://github.com/tiny-http/tiny-http/issues/220).
- TODO: Add others from <https://www.arewewebyet.org/topics/frameworks/>

## Changelog
- v0.1.0 - First published version

## TO DO
- Fix limitations above

## Release Process
1. Edit `Cargo.toml` and bump version number.
1. Run `./release.sh`

License: MIT OR Apache-2.0
