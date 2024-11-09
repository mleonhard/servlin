mod test_util;
use crate::test_util::check_elapsed;
use permit::Permit;
use safina::executor::Executor;
use servlin::{socket_addr_127_0_0_1_any_port, HttpServerBuilder, Response};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn drop_connection() {
    safina::timer::start_timer_thread();
    let permit = Permit::new();
    let handler = |_req| Response::drop_connection();
    let executor: Arc<Executor> = Executor::new(1, 1).unwrap();
    let (addr, _stopped_receiver) = executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1_any_port())
                .max_conns(10)
                .small_body_len(64 * 1024)
                .permit(permit.new_sub())
                .spawn(handler),
        )
        .unwrap();
    let before = Instant::now();
    // Note that macOS limits total number of sockets to 16383, including those in TIME_WAIT state.
    // Above that number, the connect call will block.
    for n in 0..1000 {
        let mut tcp_stream =
            std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(500))
                .unwrap_or_else(|_| panic!("attempt {n}"));
        tcp_stream.write_all(b"M / HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(0, tcp_stream.read(&mut [0u8; 1]).unwrap());
    }
    check_elapsed(before, 0..1_000).unwrap();
}

#[test]
fn client_timeout() {
    safina::timer::start_timer_thread();
    let apex_permit = Permit::new();
    let handler = |_req| {
        std::thread::sleep(Duration::from_secs(1));
        Response::text(200, "body1")
    };
    let executor: Arc<Executor> = Executor::new(1, 10).unwrap();
    let (addr, _stopped_receiver) = executor
        .block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1_any_port())
                .max_conns(100)
                .small_body_len(64 * 1024)
                .permit(apex_permit.new_sub())
                .spawn(handler),
        )
        .unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    let threads_permit = apex_permit.new_sub();
    let mut thread_handles = Vec::new();
    for _ in 0..10 {
        let permit = threads_permit.new_sub();
        let addr_clone = addr;
        let counter_clone = Arc::clone(&counter);
        let handle = std::thread::spawn(move || {
            while !permit.is_revoked() {
                let mut tcp_stream =
                    std::net::TcpStream::connect_timeout(&addr_clone, Duration::from_millis(1000))
                        .unwrap();
                tcp_stream.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
                tcp_stream
                    .set_read_timeout(Some(Duration::from_millis(50)))
                    .unwrap();
                tcp_stream.read_to_string(&mut String::new()).unwrap_err();
                counter_clone.fetch_add(1, Ordering::AcqRel);
            }
        });
        thread_handles.push(handle);
    }
    std::thread::sleep(Duration::from_millis(1000));
    drop(threads_permit);
    for handle in thread_handles {
        handle.join().unwrap();
    }
    let count = counter.load(Ordering::Acquire);
    assert!(count > 100, "count={count}");
    println!("count={count}");
}
