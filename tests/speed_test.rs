mod test_util;
use crate::test_util::check_elapsed;
use permit::Permit;
use safina::executor::Executor;
use servlin::{socket_addr_127_0_0_1_any_port, HttpServerBuilder, Response};
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn drop_connection() {
    safina::timer::start_timer_thread();
    let permit = Permit::new();
    let handler = |_req| Response::drop_connection();
    let executor: Arc<Executor> = Executor::new(4, 4).unwrap();
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
    let expected_rps = 3000;
    // Note that macOS limits total number of sockets to 16383, including those in TIME_WAIT state.
    // Above that number, the connect call will block.
    for n in 0..(3 * expected_rps) {
        let mut tcp_stream =
            std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(60_000))
                .expect(&format!("attempt {n}"));
        tcp_stream.write_all(b"M / HTTP/1.1\r\n\r\n").unwrap();
        assert_eq!(0, tcp_stream.read(&mut [0u8; 1]).unwrap());
    }
    check_elapsed(before, 0..3_000).unwrap();
}
