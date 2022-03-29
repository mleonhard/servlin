//! JSON-RPC API Server Example
//! ===========================
//!
//! Start the server:
//! ```
//! cargo run --package beatrice --features json --example json_api
//!    Compiling beatrice v0.1.0 (/x/beatrice-rs)
//!     Finished dev [unoptimized + debuginfo] target(s) in 2.20s
//!      Running `target/debug/examples/json_api`
//! INFO GET /get => 200 len=11
//! INFO POST /increment => 200 len=11
//! INFO POST /add => 200 len=11
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
//! {"count":1}
//! $ echo -ne 'POST /add HTTP/1.1\r\nContent-type:application/json\r\nContent-length:9\r\n\r\n{"num":3}' |nc 127.0.0.1 8000
//! HTTP/1.1 200 OK
//! content-type: application/json; charset=UTF-8
//! content-length: 11
//!
//! {"count":4}
//! ```
use beatrice::reexport::{safina_executor, safina_timer};
use beatrice::{print_log_response, socket_addr_127_0_0_1, HttpServerBuilder, Request, Response};
use serde::Deserialize;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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

fn get_count(state: &Arc<State>) -> Response {
    Response::json(200, json!({ "count": state.get() })).unwrap()
}

fn increment(state: &Arc<State>) -> Response {
    state.increment();
    Response::json(200, json!({ "count": state.get() })).unwrap()
}

fn add(state: &Arc<State>, req: &Request) -> Result<Response, Response> {
    #[derive(Deserialize)]
    struct Input {
        num: usize,
    }
    let input: Input = req.json()?;
    let num = if input.num > 5 {
        return Err(Response::text(400, "num is too big"));
    } else {
        input.num
    };
    state.add(num);
    Ok(Response::json(200, json!({ "count": state.get() })).unwrap())
}

fn handle_req(state: &Arc<State>, req: &Request) -> Result<Response, Response> {
    match (req.method(), req.url().path()) {
        ("GET", "/health") => Ok(Response::text(200, "ok")),
        ("GET", "/get") => Ok(get_count(state)),
        ("POST", "/increment") => Ok(increment(state)),
        ("POST", "/add") => add(state, req),
        _ => Ok(Response::text(404, "Not found")),
    }
}

pub fn main() {
    safina_timer::start_timer_thread();
    let executor = safina_executor::Executor::default();
    let state = Arc::new(State::new());
    let request_handler = move |req: Request| {
        print_log_response(
            req.method().to_string(),
            req.url().clone(),
            handle_req(&state, &req),
        )
    };
    executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1(8000))
                .max_conns(100)
                .spawn_and_join(request_handler),
        )
        .unwrap();
}
