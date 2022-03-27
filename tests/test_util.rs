#![cfg(feature = "internals")]
#![allow(dead_code)]

use beatrice::internals::listen_127_0_0_1_any_port;
use beatrice::{socket_addr_127_0_0_1_any_port, HttpServerBuilder, Request, Response};
use permit::Permit;
use safina_executor::Executor;
use safina_sync::Receiver;
use std::future::Future;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Arc;
use std::time::Duration;
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
        ExchangeErr::Connect(e.kind(), format!("{:?}", e))
    }
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn write(e: std::io::Error) -> Self {
        ExchangeErr::Write(e.kind(), format!("{:?}", e))
    }
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn read(e: std::io::Error) -> Self {
        ExchangeErr::Read(e.kind(), format!("{:?}", e))
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn unwrap_connect(self) {
        assert!(
            matches!(self, ExchangeErr::Connect(..)),
            "unwrap_connect called on {:?}",
            self
        );
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn unwrap_write(self) {
        assert!(
            matches!(self, ExchangeErr::Write(..)),
            "unwrap_write called on {:?}",
            self
        );
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn unwrap_read(self) {
        assert!(
            matches!(self, ExchangeErr::Read(..)),
            "unwrap_read called on {:?}",
            self
        );
    }
}

pub struct TestServer {
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
            executor,
            addr,
            opt_permit: Some(permit),
            opt_stopped_receiver: Some(stopped_receiver),
        })
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
        tcp_stream
            .read_to_string(&mut string)
            .map_err(ExchangeErr::read)?;
        Ok(string)
    }
}
impl Drop for TestServer {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        println!("TestServer::Drop");
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
    let listener = listen_127_0_0_1_any_port().await.unwrap();
    let listen_addr = listener.local_addr().unwrap();
    let (sender, mut receiver) = safina_sync::oneshot();
    safina_executor::spawn(async move {
        let _result = sender.send(listener.accept().await.unwrap().0);
    });
    let stream0 = async_net::TcpStream::connect(listen_addr).await.unwrap();
    let stream1 = receiver.async_recv().await.unwrap();
    (stream0, stream1)
}
