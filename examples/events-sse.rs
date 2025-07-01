//! Server-Sent Events Example
//! ==========================
//!
//! Start the server:
//! ```
//! $ cargo run --package servlin --example events-sse
//!     Finished dev [unoptimized + debuginfo] target(s) in 0.12s
//!      Running `target/debug/examples/events-sse`
//! Access the server at http://127.0.0.1:8000/subscribe
//! 2023-04-20T17:17:04Z info "code":200,"http_method":"GET","path":"/subscribe","request_id":3849933270728705814
//! ^C
//! ```
//!
//! Make a request to it:
//! ```
//! $ curl http://127.0.0.1:8000/subscribe
//! data: 2
//! data: 3
//! data: 4
//! data: 5
//! $
//! ```
#![forbid(unsafe_code)]
use permit::Permit;
use safina::executor::Executor;
use servlin::log::log_request_and_response;
use servlin::{
    Error, Event, EventSender, HttpServerBuilder, Request, Response, socket_addr_127_0_0_1,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct State {
    subscribers: Mutex<Vec<EventSender>>,
}
impl State {
    pub fn new() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn event_sender_thread(state: Arc<State>, permit: Permit) {
    loop {
        for n in 0..6 {
            std::thread::sleep(Duration::from_secs(1));
            // TODO: Rename Permit::wait and related functions to clarify that they are waiting for
            //       subordinates to drop.
            // TODO: Add a method to wait for a permit to be revoked, and another with a timeout.
            if permit.is_revoked() {
                return;
            }
            for subscriber in state.subscribers.lock().unwrap().iter_mut() {
                subscriber.send(Event::Message(n.to_string()));
            }
        }
        state.subscribers.lock().unwrap().clear();
    }
}

#[allow(clippy::unnecessary_wraps)]
#[allow(clippy::needless_pass_by_value)]
fn subscribe(state: Arc<State>, _req: Request) -> Result<Response, Error> {
    let (sender, response) = Response::event_stream();
    state.subscribers.lock().unwrap().push(sender);
    Ok(response)
}

#[allow(clippy::unnecessary_wraps)]
fn handle_req(state: Arc<State>, req: Request) -> Result<Response, Error> {
    match (req.method(), req.url().path()) {
        ("GET", "/health") => Ok(Response::text(200, "ok")),
        ("GET", "/subscribe") => subscribe(state, req),
        _ => Ok(Response::text(404, "Not found")),
    }
}

pub fn main() {
    println!("Access the server at http://127.0.0.1:8000/subscribe");
    let event_sender_thread_permit = Permit::new();
    let state = Arc::new(State::new());
    let state_clone = state.clone();
    std::thread::spawn(move || event_sender_thread(state_clone, event_sender_thread_permit));
    safina::timer::start_timer_thread();
    let executor: Arc<Executor> = Arc::default();
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
