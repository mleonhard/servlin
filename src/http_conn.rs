use crate::http_error::HttpError;
use crate::request::read_http_request;
use crate::request_body::{
    read_http_body_to_file, read_http_body_to_vec, read_http_unsized_body_to_file,
    read_http_unsized_body_to_vec,
};
use crate::response::{write_http_response, ResponseKind};
use crate::token_set::Token;
use crate::util::AsyncWriteCounter;
use crate::{Request, RequestBody, Response};
use fixed_buffer::FixedBuf;
use futures_lite::AsyncReadExt;
use permit::Permit;
use std::convert::TryFrom;
use std::future::Future;
use std::net::{Shutdown, SocketAddr};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadState {
    Head,
    /// Body(len: Option<u64>, expect_continue: bool, chunked: bool, gzip: bool)
    Body(Option<u64>, bool, bool, bool),
    Shutdown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteState {
    None,
    Response,
    Shutdown,
}

pub struct HttpConn {
    pub remote_addr: SocketAddr,
    pub buf: FixedBuf<8192>,
    pub stream: async_net::TcpStream,
    pub read_state: ReadState,
    pub write_state: WriteState,
}
impl HttpConn {
    #[must_use]
    pub fn new(remote_addr: SocketAddr, stream: async_net::TcpStream) -> Self {
        Self {
            remote_addr,
            buf: FixedBuf::new(),
            stream,
            read_state: ReadState::Head,
            write_state: WriteState::None,
        }
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.read_state == ReadState::Head
    }

    pub fn shutdown_write(&mut self) {
        //dbg!("shutdown_write");
        let _ignored = self.stream.shutdown(Shutdown::Write);
        self.write_state = WriteState::Shutdown;
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn shutdown_write_on_err<T, E>(&mut self, result: Result<T, E>) -> Result<T, E> {
        if result.is_err() {
            self.shutdown_write();
        }
        result
    }

    /// # Errors
    /// Returns an error when:
    /// - we did not send a response to the previous request
    /// - the connection is closed
    /// - we did not read the response body of the previous request
    /// - we fail to read the request
    /// - we fail to parse the request
    pub async fn read_request(&mut self) -> Result<Request, HttpError> {
        //dbg!("read_request");
        match self.write_state {
            WriteState::None => {}
            WriteState::Response => return Err(HttpError::ResponseNotSent),
            WriteState::Shutdown => return Err(HttpError::Disconnected),
        }
        match self.read_state {
            ReadState::Head => {}
            ReadState::Body(..) => return Err(HttpError::BodyNotRead),
            ReadState::Shutdown => return Err(HttpError::Disconnected),
        }
        self.write_state = WriteState::Response;
        let req = read_http_request(self.remote_addr, &mut self.buf, &mut self.stream).await?;
        self.read_state = match &req.body {
            RequestBody::PendingKnown(len) => {
                ReadState::Body(Some(*len), req.expect_continue, req.chunked, req.gzip)
            }
            RequestBody::PendingUnknown => {
                ReadState::Body(None, req.expect_continue, req.chunked, req.gzip)
            }
            _ => ReadState::Head,
        };
        Ok(req)
    }

    /// # Errors
    /// Returns an error when:
    /// - the connection is closed
    /// - a response was already sent
    /// - we fail to send the response
    pub async fn write_http_continue(&mut self) -> Result<(), HttpError> {
        match self.write_state {
            WriteState::None => return Err(HttpError::ResponseAlreadySent),
            WriteState::Response => {}
            WriteState::Shutdown => return Err(HttpError::Disconnected),
        }
        //dbg!("write_http_continue");
        self.write_response(&Response::new(100)).await
    }

    /// # Errors
    /// Returns an error when:
    /// - the client did not send a request body
    /// - the request body was already read from the client
    /// - the client used an unsupported transfer encoding
    /// - we fail to read the request body
    pub async fn read_body_to_vec(&mut self) -> Result<RequestBody, HttpError> {
        //dbg!("read_body_to_vec");
        match self.read_state {
            ReadState::Head => Err(HttpError::BodyNotAvailable),
            // TODO: Support chunked transfer encoding, as required by the HTTP/1.1 spec.
            // https://datatracker.ietf.org/doc/html/rfc7230#section-3.3.1
            ReadState::Body(_len, _expect_continue, true, _)
            | ReadState::Body(_len, _expect_continue, _, true) => {
                Err(HttpError::UnsupportedTransferEncoding)
            }
            ReadState::Body(Some(len_u64), expect_continue, false, false) => {
                let len_usize =
                    usize::try_from(len_u64).map_err(|_| HttpError::InvalidContentLength)?;
                if expect_continue {
                    self.write_http_continue().await?;
                }
                self.read_state = ReadState::Head;
                read_http_body_to_vec((&mut self.buf).chain(&mut self.stream), len_usize).await
            }
            ReadState::Body(None, expect_continue, false, false) => {
                if expect_continue {
                    self.write_http_continue().await?;
                }
                self.read_state = ReadState::Shutdown;
                read_http_unsized_body_to_vec((&mut self.buf).chain(&mut self.stream)).await
            }
            ReadState::Shutdown => Err(HttpError::Disconnected),
        }
    }

    /// # Errors
    /// Returns an error when:
    /// - the client did not send a request body
    /// - the request body was already read from the client
    /// - the client used an unsupported transfer encoding
    /// - the client sends a request body that is larger than `max_len`
    /// - we fail to read the request body
    /// - we fail to create or write the temporary file
    pub async fn read_body_to_file(
        &mut self,
        dir: &Path,
        max_len: u64,
    ) -> Result<RequestBody, HttpError> {
        //dbg!("read_body_to_file", max_len, dir);
        match self.read_state {
            ReadState::Head => Err(HttpError::BodyNotAvailable),
            // TODO: Support chunked transfer encoding, as required by the HTTP/1.1 spec.
            // https://datatracker.ietf.org/doc/html/rfc7230#section-3.3.1
            ReadState::Body(_len, _expect_continue, true, _)
            | ReadState::Body(_len, _expect_continue, _, true) => {
                Err(HttpError::UnsupportedTransferEncoding)
            }
            ReadState::Body(Some(len), _expect_continue, false, false) if len > max_len => {
                Err(HttpError::BodyTooLong)
            }
            ReadState::Body(Some(len), expect_continue, false, false) => {
                if expect_continue {
                    self.write_http_continue().await?;
                }
                self.read_state = ReadState::Head;
                read_http_body_to_file((&mut self.buf).chain(&mut self.stream), len, dir).await
            }
            ReadState::Body(None, expect_continue, false, false) => {
                if expect_continue {
                    self.write_http_continue().await?;
                }
                self.read_state = ReadState::Shutdown;
                read_http_unsized_body_to_file(
                    (&mut self.buf).chain(&mut self.stream),
                    dir,
                    max_len,
                )
                .await
            }
            ReadState::Shutdown => Err(HttpError::Disconnected),
        }
    }

    /// # Errors
    /// Returns an error when a response was already sent, the connection is closed,
    /// or it fails to send the response bytes over the network connection.
    pub async fn write_response(&mut self, response: &Response) -> Result<(), HttpError> {
        //dbg!("write_response");
        match self.write_state {
            WriteState::None => return Err(HttpError::ResponseAlreadySent),
            WriteState::Response => {}
            WriteState::Shutdown => return Err(HttpError::Disconnected),
        }
        let mut write_counter = AsyncWriteCounter::new(&mut self.stream);
        let result = write_http_response(&mut write_counter, response).await;
        if result.is_ok() {
            if !response.is_1xx() {
                self.write_state = WriteState::None;
            }
        } else if write_counter.num_bytes_written() > 0 {
            self.shutdown_write();
        }
        result
    }
}

/// # Errors
/// Returns an error when:
/// - we fail to read a request
/// - the request handler fails
/// - the request body is too long
/// - we fail to send the response
pub async fn handle_http_conn_once<F, Fut>(
    http_conn: &mut HttpConn,
    opt_cache_dir: Option<&Path>,
    small_body_len: usize,
    request_handler: F,
) -> Result<(), HttpError>
where
    Fut: Future<Output = Response>,
    F: FnOnce(Request) -> Fut + 'static + Send + Clone,
{
    //dbg!("handle_http_conn_once");
    let mut req = http_conn.read_request().await?;
    //dbg!(&req);
    if req.body.is_pending() {
        if req.body.length_is_known() && req.body.len() <= (small_body_len as u64) {
            req.body = http_conn.read_body_to_vec().await?;
            //dbg!(&req);
        } else {
            //dbg!("request_handler");
            let response = request_handler.clone()(req.clone()).await;
            //dbg!(&response);
            match response.kind {
                ResponseKind::Normal => {}
                ResponseKind::DropConnection => return Err(HttpError::Disconnected),
                ResponseKind::GetBodyAndReprocess(max_len) => {
                    let cache_dir = opt_cache_dir.ok_or(HttpError::CacheDirNotConfigured)?;
                    req.body = http_conn.read_body_to_file(cache_dir, max_len).await?;
                    //dbg!(&req);
                }
            }
        }
    }
    //dbg!("request_handler");
    let response = request_handler(req).await;
    //dbg!(&response);
    match response.kind {
        ResponseKind::Normal => {}
        ResponseKind::DropConnection => return Err(HttpError::Disconnected),
        ResponseKind::GetBodyAndReprocess(..) => return Err(HttpError::AlreadyGotBody),
    }
    if response.is_normal() && (response.is_4xx() || response.is_5xx()) {
        let _ignored = http_conn.write_response(&response).await;
        Err(HttpError::Disconnected)
    } else {
        let result = http_conn.write_response(&response).await;
        //dbg!(&result);
        result
    }
}

#[allow(clippy::module_name_repetitions)]
pub async fn handle_http_conn<F, Fut>(
    permit: Permit,
    _token: Token,
    mut http_conn: HttpConn,
    opt_cache_dir: Option<PathBuf>,
    small_body_len: usize,
    async_request_handler: F,
) where
    Fut: Future<Output = Response>,
    F: FnOnce(Request) -> Fut + 'static + Send + Clone,
{
    //dbg!("handle_http_conn");
    while !permit.is_revoked() {
        if !http_conn.is_ready() {
            // Previous request did not download body.
            return;
        }
        let result = handle_http_conn_once(
            &mut http_conn,
            opt_cache_dir.as_deref(),
            small_body_len,
            async_request_handler.clone(),
        )
        .await;
        //dbg!(&result);
        match result {
            Ok(()) => {}
            Err(HttpError::Disconnected) => return,
            Err(e) => {
                let _ignored = http_conn.write_response(&e.into()).await;
                // Disconnect clients after an error.
                // This lets connection rate limiting work for bad requests.
                http_conn.shutdown_write();
                return;
            }
        }
    }
}
