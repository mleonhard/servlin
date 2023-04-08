#![allow(dead_code)]

use permit::Permit;
use safe_regex::{Matcher0, Matcher1};
use safina_executor::Executor;
use safina_sync::Receiver;
use servlin::{socket_addr_127_0_0_1_any_port, HttpServerBuilder, Request, Response};
use std::future::Future;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr};
use std::ops::Range;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Arc;
use std::time::{Duration, Instant};
use temp_dir::TempDir;

#[allow(clippy::missing_panics_doc)]
pub fn assert_starts_with(value: impl AsRef<str>, suffix: impl AsRef<str>) {
    assert!(
        value.as_ref().starts_with(suffix.as_ref()),
        "value {:?} does not start with {:?}",
        value.as_ref(),
        suffix.as_ref()
    );
}

#[allow(clippy::missing_panics_doc)]
pub fn assert_ends_with(value: impl AsRef<str>, suffix: impl AsRef<str>) {
    assert!(
        value.as_ref().ends_with(suffix.as_ref()),
        "value {:?} does not end with {:?}",
        value.as_ref(),
        suffix.as_ref()
    );
}

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
pub fn check_elapsed(before: Instant, range_ms: Range<u64>) -> Result<(), String> {
    assert!(!range_ms.is_empty(), "invalid range {range_ms:?}");
    let elapsed = before.elapsed();
    let duration_range = Duration::from_millis(range_ms.start)..Duration::from_millis(range_ms.end);
    if duration_range.contains(&elapsed) {
        Ok(())
    } else {
        Err(format!(
            "{elapsed:?} elapsed, out of range {duration_range:?}"
        ))
    }
}

#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
pub fn read_response(tcp_stream: &mut std::net::TcpStream) -> Result<String, std::io::Error> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut bytes = Vec::new();
    loop {
        let now = Instant::now();
        if deadline < now {
            return Err(std::io::Error::new(ErrorKind::TimedOut, "timed out"));
        }
        tcp_stream.set_read_timeout(Some(deadline.duration_since(now)))?;
        let mut buf = [0_u8; 1];
        match tcp_stream.read(&mut buf) {
            Ok(0) => break,
            Ok(1) => bytes.push(buf[0]),
            Ok(_) => unreachable!(),
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                return Err(std::io::Error::new(ErrorKind::TimedOut, "timed out"))
            }
            Err(e) => return Err(e),
        }
        //dbg!(escape_ascii(bytes.as_slice()));
        if bytes.len() >= 4 && &bytes.as_slice()[(bytes.len() - 4)..] == b"\r\n\r\n".as_slice() {
            break;
        }
    }
    let head_len = bytes.len();
    //dbg!(head_len);
    let status_100_matcher: Matcher0<_> = safe_regex::regex!(br"HTTP/1.1 1.*");
    if !status_100_matcher.is_match(bytes.as_slice()) {
        //dbg!("not-100");
        #[allow(clippy::assign_op_pattern)]
        #[allow(clippy::range_plus_one)]
        let content_length_matcher: Matcher1<_> =
            safe_regex::regex!(br".*\ncontent-length:([^\r]+).*");
        if let Some((content_length_bytes,)) = content_length_matcher.match_slices(bytes.as_slice())
        {
            //dbg!(escape_ascii(content_length_bytes));
            let content_length_string: String =
                String::from_utf8(content_length_bytes.to_vec()).unwrap();
            let content_length: usize = content_length_string.trim().parse().unwrap();
            tcp_stream
                .take(content_length as u64)
                .read_to_end(&mut bytes)?;
            assert_eq!(head_len + content_length, bytes.len());
        } else {
            tcp_stream.read_to_end(&mut bytes)?;
        }
    }
    String::from_utf8(bytes)
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidData, "bytes are not UTF-8"))
}

#[allow(clippy::missing_errors_doc)]
pub fn read_to_string(reader: &mut std::net::TcpStream) -> Result<String, std::io::Error> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut bytes = Vec::new();
    loop {
        let now = Instant::now();
        let timeout = if deadline < now {
            Duration::ZERO
        } else {
            deadline.duration_since(now)
        };
        reader.set_read_timeout(Some(timeout))?;
        let mut buf = [0_u8; 1024];
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => bytes.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                return Err(std::io::Error::new(ErrorKind::TimedOut, "timed out"))
            }
            Err(e) => return Err(e),
        }
    }
    String::from_utf8(bytes)
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidData, "bytes are not UTF-8"))
}

#[allow(clippy::missing_errors_doc)]
pub fn read_for(
    reader: &mut std::net::TcpStream,
    duration_ms: u64,
) -> Result<String, std::io::Error> {
    let deadline = Instant::now() + Duration::from_millis(duration_ms);
    let mut bytes = Vec::new();
    loop {
        let now = Instant::now();
        if deadline < now {
            break;
        }
        reader.set_read_timeout(Some(deadline.duration_since(now)))?;
        let mut buf = [0_u8; 1024];
        match reader.read(&mut buf) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    ErrorKind::NotConnected,
                    "connection closed",
                ))
            }
            Ok(n) => bytes.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }
    String::from_utf8(bytes)
        .map_err(|_| std::io::Error::new(ErrorKind::InvalidData, "bytes are not UTF-8"))
}

#[allow(clippy::missing_panics_doc)]
pub fn async_test<Fut: Future<Output = ()> + Send + 'static>(fut: Fut) {
    safina_timer::start_timer_thread();
    safina_executor::Executor::new(2, 1)
        .unwrap()
        .block_on(safina_timer::with_timeout(fut, Duration::from_secs(10)))
        .unwrap();
}

#[derive(Debug, Eq, PartialEq)]
pub enum ExchangeErr {
    Connect(ErrorKind, String),
    Write(ErrorKind, String),
    Read(ErrorKind, String),
}
impl ExchangeErr {
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn connect(e: std::io::Error) -> Self {
        ExchangeErr::Connect(e.kind(), format!("{e:?}"))
    }
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn write(e: std::io::Error) -> Self {
        ExchangeErr::Write(e.kind(), format!("{e:?}"))
    }
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn read(e: std::io::Error) -> Self {
        ExchangeErr::Read(e.kind(), format!("{e:?}"))
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn unwrap_connect(self) {
        assert!(
            matches!(self, ExchangeErr::Connect(..)),
            "unwrap_connect called on {self:?}"
        );
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn unwrap_write(self) {
        assert!(
            matches!(self, ExchangeErr::Write(..)),
            "unwrap_write called on {self:?}"
        );
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn unwrap_read(self) {
        assert!(
            matches!(self, ExchangeErr::Read(..)),
            "unwrap_read called on {self:?}"
        );
    }
}

pub struct TestServer {
    pub cache_dir: Option<TempDir>,
    pub executor: Arc<Executor>,
    pub addr: SocketAddr,
    pub opt_permit: Option<Permit>,
    pub opt_stopped_receiver: Option<Receiver<()>>,
}
impl TestServer {
    #[allow(clippy::missing_errors_doc)]
    pub fn start<F>(handler: F) -> Result<Self, std::io::Error>
    where
        F: FnOnce(Request) -> Response + 'static + Clone + Send + Sync,
    {
        safina_timer::start_timer_thread();
        let permit = Permit::new();
        let executor = safina_executor::Executor::new(1, 1)?;
        let cache_dir = TempDir::new()?;
        let (addr, stopped_receiver): (SocketAddr, Receiver<()>) = executor.block_on(
            HttpServerBuilder::new()
                .listen_addr(socket_addr_127_0_0_1_any_port())
                .max_conns(1000)
                .small_body_len(64 * 1024)
                .receive_large_bodies(cache_dir.path())
                .permit(permit.new_sub())
                .spawn(handler),
        )?;
        Ok(Self {
            cache_dir: Some(cache_dir),
            executor,
            addr,
            opt_permit: Some(permit),
            opt_stopped_receiver: Some(stopped_receiver),
        })
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn connect(&self) -> Result<std::net::TcpStream, std::io::Error> {
        std::net::TcpStream::connect_timeout(&self.addr, Duration::from_millis(500))
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn connect_and_send(
        &self,
        send: impl AsRef<[u8]>,
    ) -> Result<std::net::TcpStream, ExchangeErr> {
        let mut tcp_stream =
            std::net::TcpStream::connect_timeout(&self.addr, Duration::from_millis(500))
                .map_err(ExchangeErr::connect)?;
        tcp_stream
            .write_all(send.as_ref())
            .map_err(ExchangeErr::write)?;
        Ok(tcp_stream)
    }

    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub fn exchange(&self, send: impl AsRef<[u8]>) -> Result<String, ExchangeErr> {
        let mut tcp_stream =
            std::net::TcpStream::connect_timeout(&self.addr, Duration::from_millis(500))
                .map_err(ExchangeErr::connect)?;
        tcp_stream
            .write_all(send.as_ref())
            .map_err(ExchangeErr::write)?;
        tcp_stream.shutdown(Shutdown::Write).unwrap();
        let mut string = String::new();
        match tcp_stream.read_to_string(&mut string) {
            Ok(_) => Ok(string),
            Err(e) => Err(ExchangeErr::read(e)),
        }
    }
}
impl Drop for TestServer {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        self.opt_permit.take();
        if let Some(stopped_receiver) = self.opt_stopped_receiver.take() {
            match stopped_receiver.recv_timeout(Duration::from_secs(5)) {
                Err(RecvTimeoutError::Timeout) => panic!("timed out waiting for server to stop"),
                Err(RecvTimeoutError::Disconnected) => panic!("server crashed"),
                Ok(()) => {}
            }
        }
    }
}

#[allow(clippy::missing_panics_doc)]
pub async fn connected_streams() -> (async_net::TcpStream, async_net::TcpStream) {
    let listener = async_net::TcpListener::bind(socket_addr_127_0_0_1_any_port())
        .await
        .unwrap();
    let listen_addr = listener.local_addr().unwrap();
    let (sender, mut receiver) = safina_sync::oneshot();
    safina_executor::spawn(async move {
        let _result = sender.send(listener.accept().await.unwrap().0);
    });
    let stream0 = async_net::TcpStream::connect(listen_addr).await.unwrap();
    let stream1 = receiver.async_recv().await.unwrap();
    (stream0, stream1)
}
