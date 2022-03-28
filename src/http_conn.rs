use crate::body::{
    read_http_body_to_file, read_http_body_to_vec, read_http_unsized_body_to_file,
    read_http_unsized_body_to_vec, write_http_continue,
};
use crate::http_error::HttpError;
use crate::request::read_http_request;
use crate::response::write_http_response;
use crate::token_set::Token;
use crate::util::AsyncWriteCounter;
use crate::{Body, Request, Response};
use fixed_buffer::FixedBuf;
use futures_lite::AsyncReadExt;
use permit::Permit;
use std::convert::TryFrom;
use std::future::Future;
use std::net::{Shutdown, SocketAddr};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadState {
    Ready,
    Bytes(u64),
    Chunks,
    Unknown,
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
    pub expect_continue: bool,
    pub write_state: WriteState,
}
impl HttpConn {
    #[must_use]
    pub fn new(remote_addr: SocketAddr, stream: async_net::TcpStream) -> Self {
        Self {
            remote_addr,
            buf: FixedBuf::new(),
            stream,
            read_state: ReadState::Ready,
            expect_continue: false,
            write_state: WriteState::None,
        }
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.read_state == ReadState::Ready
    }

    pub fn shutdown(&mut self) {
        self.shutdown_read();
        self.shutdown_write();
    }

    pub fn shutdown_read(&mut self) {
        //dbg!("shutdown_read");
        let _ignored = self.stream.shutdown(Shutdown::Read);
        self.read_state = ReadState::Shutdown;
    }

    pub fn shutdown_write(&mut self) {
        //dbg!("shutdown_write");
        let _ignored = self.stream.shutdown(Shutdown::Write);
        self.write_state = WriteState::Shutdown;
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn shutdown_read_on_err<T, E>(&mut self, result: Result<T, E>) -> Result<T, E> {
        if result.is_err() {
            self.shutdown_read();
        }
        result
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn shutdown_on_err<T, E>(&mut self, result: Result<T, E>) -> Result<T, E> {
        if result.is_err() {
            self.shutdown_read();
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
        let result = {
            //dbg!("read_request");
            match self.write_state {
                WriteState::None => {}
                WriteState::Response => return Err(HttpError::ResponseNotSent),
                WriteState::Shutdown => return Err(HttpError::Disconnected),
            }
            match self.read_state {
                ReadState::Ready => {}
                ReadState::Bytes(..) | ReadState::Chunks | ReadState::Unknown => {
                    return Err(HttpError::BodyNotRead);
                }
                ReadState::Shutdown => return Err(HttpError::Disconnected),
            }
            self.write_state = WriteState::Response;
            let req = read_http_request(self.remote_addr, &mut self.buf, &mut self.stream).await?;
            if req.body().is_pending() {
                // This code is complicated because HTTP/1.1 defines 3 ways to frame a body and
                // complicated rules for deciding which framing method to expect:
                // https://datatracker.ietf.org/doc/html/rfc7230#section-3.3
                if req.chunked {
                    self.read_state = ReadState::Chunks;
                } else if req.content_length == Some(0) {
                    self.read_state = ReadState::Ready;
                } else if let Some(content_length) = &req.content_length {
                    self.read_state = ReadState::Bytes(*content_length);
                } else {
                    match req.method() {
                        "POST" | "PUT" => self.read_state = ReadState::Unknown,
                        _ => self.read_state = ReadState::Ready,
                    }
                };
            } else {
                self.read_state = ReadState::Ready;
            }
            Ok(req)
        };
        self.shutdown_read_on_err(result)
    }

    /// # Errors
    /// Returns an error when:
    /// - the connection is closed
    /// - a response was already sent
    /// - we fail to send the response
    pub async fn write_http_continue_if_needed(&mut self) -> Result<(), HttpError> {
        match self.write_state {
            WriteState::None => return Err(HttpError::ResponseAlreadySent),
            WriteState::Response => {}
            WriteState::Shutdown => return Err(HttpError::Disconnected),
        }
        let result = {
            //dbg!("write_http_continue_if_needed");
            if self.expect_continue {
                write_http_continue(&mut self.stream).await?;
                self.expect_continue = false;
            }
            Ok(())
        };
        self.shutdown_on_err(result)
    }

    #[must_use]
    pub fn has_body(&self) -> bool {
        match self.read_state {
            ReadState::Ready | ReadState::Shutdown => false,
            ReadState::Bytes(..) | ReadState::Chunks | ReadState::Unknown => true,
        }
    }

    /// # Errors
    /// Returns an error when:
    /// - the client did not send a request body
    /// - the request body was already read from the client
    /// - the client used an unsupported transfer encoding
    /// - we fail to read the request body
    pub async fn read_body_to_vec(&mut self) -> Result<Body, HttpError> {
        let result = {
            //dbg!("read_body_to_vec");
            match self.read_state {
                ReadState::Ready => return Err(HttpError::BodyNotAvailable),
                ReadState::Bytes(len_u64) => {
                    let len_usize =
                        usize::try_from(len_u64).map_err(|_| HttpError::InvalidContentLength)?;
                    self.write_http_continue_if_needed().await?;
                    self.read_state = ReadState::Ready;
                    let result =
                        read_http_body_to_vec((&mut self.buf).chain(&mut self.stream), len_usize)
                            .await;
                    result
                }
                ReadState::Unknown => {
                    self.write_http_continue_if_needed().await?;
                    self.read_state = ReadState::Shutdown;
                    read_http_unsized_body_to_vec((&mut self.buf).chain(&mut self.stream)).await
                }
                // TODO: Support chunked transfer encoding, as required by the HTTP/1.1 spec.
                // https://datatracker.ietf.org/doc/html/rfc7230#section-3.3.1
                ReadState::Chunks => Err(HttpError::UnsupportedTransferEncoding),
                ReadState::Shutdown => Err(HttpError::Disconnected),
            }
        };
        self.shutdown_read_on_err(result)
    }

    /// # Errors
    /// Returns an error when:
    /// - the client did not send a request body
    /// - the request body was already read from the client
    /// - the client used an unsupported transfer encoding
    /// - the client sends a request body that is larger than `max_len`
    /// - we fail to read the request body
    /// - we fail to create or write the temporary file
    pub async fn read_body_to_file(&mut self, dir: &Path, max_len: u64) -> Result<Body, HttpError> {
        let result = {
            //dbg!("read_body_to_file", max_len, dir);
            match self.read_state {
                ReadState::Ready => return Err(HttpError::BodyNotAvailable),
                ReadState::Bytes(len) => {
                    if len < max_len {
                        return Err(HttpError::BodyTooLong);
                    }
                    self.write_http_continue_if_needed().await?;
                    self.read_state = ReadState::Ready;
                    read_http_body_to_file((&mut self.buf).chain(&mut self.stream), len, dir).await
                }
                ReadState::Unknown => {
                    self.write_http_continue_if_needed().await?;
                    self.read_state = ReadState::Shutdown;
                    read_http_unsized_body_to_file(
                        (&mut self.buf).chain(&mut self.stream),
                        dir,
                        max_len,
                    )
                    .await
                }
                // TODO: Support chunked transfer encoding, as required by the HTTP/1.1 spec.
                // https://datatracker.ietf.org/doc/html/rfc7230#section-3.3.1
                ReadState::Chunks => Err(HttpError::UnsupportedTransferEncoding),
                ReadState::Shutdown => Err(HttpError::Disconnected),
            }
        };
        self.shutdown_read_on_err(result)
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
        match write_http_response(&mut write_counter, response).await {
            Ok(()) => {
                self.write_state = WriteState::None;
                Ok(())
            }
            Err(e) => {
                if write_counter.num_bytes_written() > 0 {
                    self.shutdown_write();
                }
                Err(e)
            }
        }
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
    if req.body.is_pending() && req.body.len() <= (small_body_len as u64) {
        req.body = http_conn.read_body_to_vec().await?;
    }
    //dbg!("request_handler", &req);
    let response = request_handler.clone()(req).await;
    //dbg!(&response);
    let response = match response {
        Response::GetBodyAndReprocess(max_len, mut req) => {
            if !req.body().is_pending() {
                return Err(HttpError::AlreadyGotBody);
            }
            let cache_dir = opt_cache_dir.ok_or(HttpError::CacheDirNotConfigured)?;
            if max_len < req.body.len() {
                //dbg!("returning HttpError::BodyTooLong");
                return Err(HttpError::BodyTooLong);
            }
            req.body = http_conn.read_body_to_file(cache_dir, max_len).await?;
            //dbg!("request_handler", &req);
            match request_handler.clone()(req).await {
                Response::GetBodyAndReprocess(..) => return Err(HttpError::AlreadyGotBody),
                Response::Drop => return Err(HttpError::Disconnected),
                normal_response @ Response::Normal(..) => normal_response,
            }
        }
        Response::Drop => return Err(HttpError::Disconnected),
        normal_response @ Response::Normal(..) => normal_response,
    };
    //dbg!(&response);
    http_conn.write_response(&response).await
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
            http_conn.shutdown();
            return;
        }
        match handle_http_conn_once(
            &mut http_conn,
            opt_cache_dir.as_deref(),
            small_body_len,
            async_request_handler.clone(),
        )
        .await
        {
            Ok(()) => {}
            Err(HttpError::Disconnected) => return,
            Err(e) => {
                if e.is_server_error() {
                    eprintln!("ERROR {:?}", e);
                }
                let _ignored = http_conn.write_response(&e.into()).await;
                return;
            }
        }
    }
}
