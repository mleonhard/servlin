//! JSON-RPC API Server Example
//! ===========================
//!
//! Start the server:
//! ```
//! % cargo run --package servlin --features json --example json_api
//!     Finished dev [unoptimized + debuginfo] target(s) in 0.12s
//!      Running `target/debug/examples/json_api`
//! Access the API at http://127.0.0.1:8000/
//! 2023-04-20T17:29:07Z info "code":200,"response_body_len":11,"http_method":"GET","path":"/get","request_id":11704830503426885018
//! 2023-04-20T17:29:17Z info "code":200,"response_body_len":11,"http_method":"POST","path":"/increment","request_id":8295975836798953203
//! 2023-04-20T17:29:17Z info "code":200,"response_body_len":11,"http_method":"POST","path":"/increment","request_id":8295975836798953203
//! 2023-04-20T17:29:23Z info "code":200,"response_body_len":11,"http_method":"POST","path":"/add","request_id":2174859481348643435
//! ^C
//! ```
//!
//! Make requests to it:
//! ```
//! $ echo -ne 'GET /get HTTP/1.1\r\n\r\n' |nc 127.0.0.1 8000
//! HTTP/1.1 200 OK
//! content-type: application/json; charset=UTF-8
//! content-length: 11
//!
//! {"count":0}
//! $ echo -ne 'POST /increment HTTP/1.1\r\n\r\n' |nc 127.0.0.1 8000
//! HTTP/1.1 200 OK
//! content-type: application/json; charset=UTF-8
//! content-length: 11
//!
//! {"count":2}
//! $ echo -ne 'POST /add HTTP/1.1\r\nContent-type:application/json\r\nContent-length:9\r\n\r\n{"num":3}' |nc 127.0.0.1 8000
//! HTTP/1.1 200 OK
//! content-type: application/json; charset=UTF-8
//! content-length: 11
//!
//! {"count":5}
//! ```
#![forbid(unsafe_code)]
use safina::executor::Executor;
use serde::Deserialize;
use serde_json::json;
use servlin::log::log_request_and_response;
use servlin::{Error, HttpServerBuilder, Request, Response, socket_addr_127_0_0_1};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct State {
    count: AtomicUsize,
}
impl State {
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    pub fn get(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    pub fn increment(&self) {
        self.count.fetch_add(1, Ordering::AcqRel);
    }

    pub fn add(&self, n: usize) {
        self.count.fetch_add(n, Ordering::AcqRel);
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_count(state: Arc<State>) -> Response {
    Response::json(200, json!({ "count": state.get() })).unwrap()
}

#[allow(clippy::needless_pass_by_value)]
fn increment(state: Arc<State>) -> Response {
    state.increment();
    Response::json(200, json!({ "count": state.get() })).unwrap()
}

#[allow(clippy::needless_pass_by_value)]
fn add(state: Arc<State>, req: Request) -> Result<Response, Error> {
    #[derive(Deserialize)]
    struct Input {
        num: usize,
    }
    let input: Input = req.json()?;
    let num = if input.num > 5 {
        return Err(Error::client_error(Response::text(400, "num is too big")));
    } else {
        input.num
    };
    state.add(num);
    Response::json(200, json!({ "count": state.get() }))
}

fn handle_req(state: Arc<State>, req: Request) -> Result<Response, Error> {
    match (req.method(), req.url().path.as_str()) {
        ("GET", "/health") => Ok(Response::text(200, "ok")),
        ("GET", "/get") => Ok(get_count(state)),
        ("POST", "/increment") => Ok(increment(state)),
        ("POST", "/add") => add(state, req),
        _ => Ok(Response::text(404, "Not found")),
    }
}

pub fn main() {
    println!("Access the API at http://127.0.0.1:8000/");
    safina::timer::start_timer_thread();
    let executor: Arc<Executor> = Arc::default();
    let state = Arc::new(State::new());
    let request_handler =
        move |req: Request| log_request_and_response(req, |req| handle_req(state, req)).unwrap();
    executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1(8000))
                .max_conns(100)
                .spawn_and_join(request_handler),
        )
        .unwrap();
}
