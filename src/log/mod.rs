mod log_file_writer;
mod logger;
mod prefix_file_set;
mod tag;
mod tag_list;
mod tag_value;

use crate::error::Error;
use crate::{Request, Response};
#[allow(clippy::module_name_repetitions)]
pub use log_file_writer::LogFileWriter;
use logger::log;
pub use logger::set_global_logger;
pub use logger::{
    add_thread_local_log_tag, add_thread_local_log_tags_from_request, clear_thread_local_log_tags,
    with_thread_local_log_tags, LoggerStoppedError,
};
use std::fmt::{Display, Formatter};
use std::time::SystemTime;
pub use tag::tag;
use tag::Tag;
pub use tag_list::*;

pub mod internal {
    pub use crate::log::log_file_writer::*;
    pub use crate::log::logger::*;
    pub use crate::log::prefix_file_set::*;
    pub use crate::log::tag::*;
    pub use crate::log::tag_value::*;
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Level {
    Error,
    Info,
    Debug,
}
impl Display for Level {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Level::Error => write!(f, "error"),
            Level::Info => write!(f, "info"),
            Level::Debug => write!(f, "debug"),
        }
    }
}

/// Make a new log event with 'error' level and sends it to the global logger.
///
/// # Errors
/// Returns `Err` when the global logger has stopped.
pub fn error(msg: impl Into<String>, tags: impl Into<TagList>) -> Result<(), LoggerStoppedError> {
    let mut tags = tags.into();
    tags.insert(0, tag("msg", msg.into()));
    log(SystemTime::now(), Level::Error, tags.into_vec())
}

/// Make a new log event with 'info' level and sends it to the global logger.
///
/// # Errors
/// Returns `Err` when the global logger has stopped.
pub fn info(msg: impl Into<String>, tags: impl Into<TagList>) -> Result<(), LoggerStoppedError> {
    let mut tags = tags.into();
    tags.insert(0, tag("msg", msg.into()));
    log(SystemTime::now(), Level::Info, tags.into_vec())
}

/// Make a new log event with 'debug' level and sends it to the global logger.
///
/// # Errors
/// Returns `Err` when the global logger has stopped.
pub fn debug(msg: impl Into<String>, tags: impl Into<TagList>) -> Result<(), LoggerStoppedError> {
    let mut tags = tags.into();
    tags.insert(0, tag("msg", msg.into()));
    log(SystemTime::now(), Level::Debug, tags.into_vec())
}

/// Makes a new log event for `result` and sends it to the global logger.
/// Returns the response.
///
/// When `result` is an [`Error`] without a response,
/// this function uses [`Response::internal_server_errror_500`] to make one.
///
/// # Errors
/// Returns `Err` when the global logger has stopped.
#[allow(clippy::module_name_repetitions)]
pub fn log_response(result: Result<Response, Error>) -> Result<Response, LoggerStoppedError> {
    match result {
        Ok(response) => {
            let mut tags = Vec::new();
            tags.push(Tag::new("code", response.code));
            if let Some(body_len) = response.body.len() {
                tags.push(Tag::new("response_body_len", body_len));
            }
            log(SystemTime::now(), Level::Info, tags)?;
            Ok(response)
        }
        Err(e) => {
            let response = e
                .response
                .unwrap_or_else(Response::internal_server_errror_500);
            let mut tags = e.tags;
            if let Some(msg) = e.msg {
                tags.push(Tag::new("msg", msg));
            }
            if let Some(backtrace) = e.backtrace {
                tags.push(Tag::new("msg", format!("{backtrace:?}")));
            }
            tags.push(Tag::new("code", response.code));
            if let Some(body_len) = response.body.len() {
                tags.push(Tag::new("response_body_len", body_len));
            }
            log(e.time, Level::Error, tags)?;
            Ok(response)
        }
    }
}

/// Adds thread-local log tags from the request and then calls the handler `f`.
/// When `f` does logging, the log messages will include the request id, HTTP method, and path.
/// When `f` returns, this function makes a new log event for the result
/// and sends it to the global logger.
///
/// When the result of `f` is an [`Error`] without a response,
/// this function uses [`Response::internal_server_errror_500`] to make one.
///
/// Returns the response.
///
/// Clears thread-local log tags.
///
/// # Errors
/// Returns `Err` when the global logger has stopped.
///
/// # Panics
/// Panics when the global logger has stopped ([`LoggerStoppedError`]).
#[allow(clippy::module_name_repetitions)]
pub fn log_request_and_response<F: FnOnce(Request) -> Result<Response, Error>>(
    req: Request,
    f: F,
) -> Response {
    clear_thread_local_log_tags();
    add_thread_local_log_tags_from_request(&req);
    let response = log_response(f(req)).unwrap();
    clear_thread_local_log_tags();
    response
}
