use crate::log::internal::{Tag, TagValue};
use crate::Response;
use std::backtrace::Backtrace;
use std::time::SystemTime;

#[derive(Debug)]
pub struct Error {
    pub msg: Option<String>,
    pub tags: Vec<Tag>,
    pub response: Option<Response>,
    pub time: SystemTime,
    pub backtrace: Option<Backtrace>,
}
impl Error {
    #[must_use]
    pub fn new() -> Self {
        Self {
            msg: None,
            tags: Vec::new(),
            response: None,
            time: SystemTime::now(),
            backtrace: None,
        }
    }

    #[must_use]
    pub fn server_error(msg: impl Into<String>) -> Self {
        Self::new().with_msg(msg.into()).with_backtrace()
    }

    #[must_use]
    pub fn client_error(response: Response) -> Self {
        Self::new().with_response(response)
    }

    #[must_use]
    pub fn with_backtrace(mut self) -> Self {
        self.backtrace = Some(Backtrace::capture());
        self
    }

    #[must_use]
    pub fn with_msg(mut self, msg: impl Into<String>) -> Self {
        let msg = msg.into();
        self.msg = if let Some(prev_msg) = self.msg {
            Some(format!("{msg}: {prev_msg}"))
        } else {
            Some(msg)
        };
        self
    }

    #[must_use]
    pub fn with_response(mut self, response: Response) -> Self {
        self.response = Some(response);
        self
    }

    #[must_use]
    pub fn with_tag(mut self, name: &'static str, value: impl Into<TagValue>) -> Self {
        self.tags.push(Tag::new(name, value));
        self
    }
}
impl From<Response> for Error {
    fn from(value: Response) -> Self {
        Self::client_error(value)
    }
}
impl From<&'_ str> for Error {
    fn from(value: &'_ str) -> Self {
        Self::server_error(value)
    }
}
impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::server_error(value)
    }
}
impl From<&dyn std::error::Error> for Error {
    fn from(value: &dyn std::error::Error) -> Self {
        Self::server_error(value.to_string())
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::server_error(value.to_string())
    }
}
impl Default for Error {
    fn default() -> Self {
        Self::new()
    }
}
impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.msg == other.msg
            && self.tags.as_slice() == other.tags.as_slice()
            && self.response == other.response
        // Do not compare backtraces or time.
    }
}
impl Eq for Error {}
