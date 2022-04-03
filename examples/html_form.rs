//! HTML Form Example
//! =================
//!
//! Start the server:
//! ```
//! % cargo run --package beatrice --features urlencoded --example html_form
//!    Compiling beatrice v0.1.0 (/x/beatrice-rs)
//!     Finished dev [unoptimized + debuginfo] target(s) in 2.35s
//!      Running `target/debug/examples/html_form`
//! Access the server at http://127.0.0.1:8000/
//! INFO GET / => 200 len=370
//! INFO POST /increment => 303 len=0
//! INFO GET / => 200 len=370
//! INFO POST /increment => 303 len=0
//! INFO GET / => 200 len=370
//! INFO POST /add => 303 len=0
//! INFO GET / => 200 len=370
//! ^C
//! ```
//!
//! Access the form with your web browser:
//! <http://127.0.0.1:8000/>
#![forbid(unsafe_code)]
use beatrice::reexport::{safina_executor, safina_timer};
use beatrice::{print_log_response, socket_addr_127_0_0_1, HttpServerBuilder, Request, Response};
use serde::Deserialize;
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

fn index(state: &Arc<State>) -> Response {
    Response::html(
        200,
        format!(
            "<html>
<head><title>Example</title></head>
<body>
  <h1>Example</h1>
  <p>Count: {}</p>
  <form action='/increment' method='post'>
    <input type='submit' value='Increment'/><br/>
  </form>
  <form action='/add' method='post'>
    <label>Num <input type='number' autofocus name='num' /></label>
    <input type='submit' name='add' value='Add'/>
  </form>
</body>
</html>",
            state.get()
        ),
    )
}

fn increment(state: &Arc<State>) -> Response {
    state.increment();
    Response::redirect_303("/")
}

fn add(state: &Arc<State>, req: &Request) -> Result<Response, Response> {
    #[derive(Deserialize)]
    struct Input {
        num: usize,
    }
    let input: Input = req.urlencoded()?;
    let num = if input.num > 5 {
        return Err(Response::text(400, "num is too big"));
    } else {
        input.num
    };
    state.add(num);
    Ok(Response::redirect_303("/"))
}

fn handle_req(state: &Arc<State>, req: &Request) -> Result<Response, Response> {
    match (req.method(), req.url().path()) {
        ("GET", "/health") => Ok(Response::text(200, "ok")),
        ("GET", "/") => Ok(index(state)),
        ("POST", "/increment") => Ok(increment(state)),
        ("POST", "/add") => add(state, req),
        _ => Ok(Response::text(404, "Not found")),
    }
}

pub fn main() {
    println!("Access the server at http://127.0.0.1:8000/");
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
