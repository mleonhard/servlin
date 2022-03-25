#![allow(dead_code)]
use crate::token_set::{Token, TokenSet};
use async_net::TcpListener;
use futures_lite::FutureExt;
use permit::Permit;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

#[must_use]
pub fn socket_addr_127_0_0_1_any_port() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
}

#[must_use]
pub fn socket_addr_127_0_0_1(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}

#[must_use]
pub fn socket_addr_all_interfaces(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port)
}

/// # Errors
/// Returns an error when we fail to bind to the address.
pub async fn listen_127_0_0_1_any_port() -> Result<async_net::TcpListener, std::io::Error> {
    TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await
}

/// # Errors
/// Returns an error when we fail to bind to the address.
pub async fn listen_127_0_0_1(port: u16) -> Result<async_net::TcpListener, std::io::Error> {
    TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)).await
}

/// # Errors
/// Returns an error when we fail to bind to the address.
pub async fn listen_all_interfaces(port: u16) -> Result<async_net::TcpListener, std::io::Error> {
    TcpListener::bind(SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port)).await
}

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum AcceptResult {
    Ok(async_net::TcpStream, SocketAddr),
    TooManyOpenFiles,
    Err(std::io::Error),
}
impl AcceptResult {
    #[must_use]
    pub fn new(res: Result<(async_net::TcpStream, SocketAddr), std::io::Error>) -> Self {
        match res {
            Ok((stream, addr)) => AcceptResult::Ok(stream, addr),
            // On Unix, std translates errno EMFILE (Too many open files) into
            // ErrorKind::Other (stable) or ErrorKind::Uncategorized (unstable).
            // The docs say that we shouldn't use either of these.
            // So we check for the POSIX errno EMFILE value: 24.
            Err(e) if e.raw_os_error() == Some(24) => AcceptResult::TooManyOpenFiles,
            Err(e) => AcceptResult::Err(e),
        }
    }
}

/// Start a task to accept connections and pass them to `stream_handler`.
///
/// The task stops then `permit` is revoked.
///
/// # Panics
/// Retries when we fail to accept a connection with error `EMFILE` (Too many open files).
/// Panics on other errors.
#[allow(clippy::module_name_repetitions)]
pub async fn accept_loop<F>(
    permit: Permit,
    listener: async_net::TcpListener,
    mut token_set: TokenSet,
    conn_handler: F,
) where
    F: FnOnce(Permit, Token, async_net::TcpStream, SocketAddr) + 'static + Send + Clone,
{
    loop {
        let token = token_set.async_wait_token().await;
        if permit.is_revoked() {
            return;
        }
        match FutureExt::or(
            async { Some(AcceptResult::new(listener.accept().await)) },
            async {
                async_io::Timer::after(Duration::from_millis(500)).await;
                None
            },
        )
        .await
        {
            Some(AcceptResult::Ok(stream, addr)) => {
                conn_handler.clone()(permit.new_sub(), token, stream, addr);
            }
            Some(AcceptResult::TooManyOpenFiles) => {
                async_io::Timer::after(Duration::from_millis(500)).await;
            }
            Some(AcceptResult::Err(e)) => panic!("error accepting connection: {}", e),
            None => {}
        }
    }
}
