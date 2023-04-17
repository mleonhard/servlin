mod log_file_writer;
mod logger;
mod prefix_file_set;
mod tag;
mod tag_list;
mod tag_value;

use crate::error::Error;
use crate::{Request, Response};
pub use log_file_writer::LogFileWriter;
use logger::log;
pub use logger::set_global_logger;
pub use logger::{
    add_thread_local_log_tag, clear_thread_local_log_tags, with_thread_local_log_tags,
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

pub fn error(msg: impl Into<String>, tags: impl Into<TagList>) {
    log(
        SystemTime::now(),
        Level::Error,
        tags.into().with("msg", msg.into()).into_vec(),
    )
}

pub fn info(msg: impl Into<String>, tags: impl Into<TagList>) {
    log(
        SystemTime::now(),
        Level::Info,
        tags.into().with("msg", msg.into()).into_vec(),
    )
}

pub fn debug(msg: impl Into<String>, tags: impl Into<TagList>) {
    log(
        SystemTime::now(),
        Level::Debug,
        tags.into().with("msg", msg.into()).into_vec(),
    )
}

#[allow(clippy::needless_pass_by_value)]
#[must_use]
pub fn log_response(req: &Request, result: Result<Response, Error>) -> Response {
    match result {
        Ok(response) => {
            let mut tags = Vec::new();
            tags.push(Tag::new("http_method", req.method()));
            tags.push(Tag::new("path", req.url().path()));
            tags.push(Tag::new("code", response.code));
            if let Some(body_len) = response.body.len() {
                tags.push(Tag::new("body_len", body_len));
            }
            log(SystemTime::now(), Level::Info, tags);
            response
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
            tags.push(Tag::new("http_method", req.method()));
            tags.push(Tag::new("path", req.url().path()));
            tags.push(Tag::new("code", response.code));
            if let Some(body_len) = response.body.len() {
                tags.push(Tag::new("body_len", body_len));
            }
            log(e.time, Level::Error, tags);
            response
        }
    }
}
