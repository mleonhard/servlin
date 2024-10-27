use crate::head::HeadError;
use crate::Response;
use safina::timer::{DeadlineError, DeadlineExceededError};
use std::io::ErrorKind;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub enum HttpError {
    AlreadyGotBody,
    BodyNotAvailable,
    BodyNotRead,
    BodyNotUtf8,
    BodyTooLong,
    CacheDirNotConfigured,
    Disconnected,
    DuplicateContentLengthHeader,
    DuplicateContentTypeHeader,
    DuplicateTransferEncodingHeader,
    ErrorReadingFile(ErrorKind, String),
    ErrorReadingResponseBody(ErrorKind, String),
    ErrorSavingFile(ErrorKind, String),
    HandlerDeadlineExceeded,
    HeadTooLong,
    InvalidContentLength,
    MalformedCookieHeader,
    MalformedHeaderLine,
    MalformedPath,
    MalformedRequestLine,
    MissingRequestLine,
    ResponseAlreadySent,
    ResponseNotSent,
    TimerThreadNotStarted,
    Truncated,
    UnsupportedProtocol,
    UnsupportedTransferEncoding,
    UnwritableResponse,
}
impl HttpError {
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn error_reading_file(e: std::io::Error) -> Self {
        HttpError::ErrorReadingFile(e.kind(), e.to_string())
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn error_reading_response_body(e: std::io::Error) -> Self {
        HttpError::ErrorReadingResponseBody(e.kind(), e.to_string())
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn error_saving_file(e: std::io::Error) -> Self {
        HttpError::ErrorSavingFile(e.kind(), e.to_string())
    }

    #[must_use]
    pub fn is_server_error(&self) -> bool {
        match self {
            HttpError::AlreadyGotBody
            | HttpError::BodyNotAvailable
            | HttpError::BodyNotRead
            | HttpError::CacheDirNotConfigured
            | HttpError::DuplicateContentLengthHeader
            | HttpError::DuplicateContentTypeHeader
            | HttpError::DuplicateTransferEncodingHeader
            | HttpError::ErrorReadingFile(..)
            | HttpError::ErrorReadingResponseBody(..)
            | HttpError::ErrorSavingFile(..)
            | HttpError::HandlerDeadlineExceeded
            | HttpError::ResponseAlreadySent
            | HttpError::ResponseNotSent
            | HttpError::UnwritableResponse => true,
            HttpError::BodyNotUtf8
            | HttpError::BodyTooLong
            | HttpError::Disconnected
            | HttpError::HeadTooLong
            | HttpError::InvalidContentLength
            | HttpError::MalformedCookieHeader
            | HttpError::MalformedHeaderLine
            | HttpError::MalformedPath
            | HttpError::MalformedRequestLine
            | HttpError::MissingRequestLine
            | HttpError::TimerThreadNotStarted
            | HttpError::Truncated
            | HttpError::UnsupportedProtocol
            | HttpError::UnsupportedTransferEncoding => false,
        }
    }

    #[must_use]
    pub fn description(&self) -> String {
        match self {
            HttpError::AlreadyGotBody => "HttpError::AlreadyGotBody".to_string(),
            HttpError::BodyNotAvailable => "HttpError::BodyNotAvailable".to_string(),
            HttpError::BodyNotRead => "HttpError::BodyNotRead".to_string(),
            HttpError::BodyNotUtf8 => "HttpError::BodyNotUtf8".to_string(),
            HttpError::BodyTooLong => "HttpError::BodyTooLong".to_string(),
            HttpError::CacheDirNotConfigured => "HttpError::CacheDirNotConfigured".to_string(),
            HttpError::DuplicateContentLengthHeader => {
                "HttpError::DuplicateContentLengthHeader".to_string()
            }
            HttpError::DuplicateContentTypeHeader => {
                "HttpError::DuplicateContentTypeHeader".to_string()
            }
            HttpError::DuplicateTransferEncodingHeader => {
                "HttpError::DuplicateTransferEncodingHeader".to_string()
            }
            HttpError::Disconnected => "HttpError::Disconnected".to_string(),
            HttpError::ErrorReadingFile(kind, s) => {
                format!("HttpError::ErrorReadingFile: {kind:?}: {s}")
            }
            HttpError::ErrorReadingResponseBody(kind, s) => {
                format!("HttpError::ErrorReadingResponseBody: {kind:?}: {s}")
            }
            HttpError::ErrorSavingFile(kind, s) => {
                format!("HttpError::ErrorSavingFile: {kind:?}: {s}")
            }
            HttpError::HandlerDeadlineExceeded => "HttpError::HandlerDeadlineExceeded".to_string(),
            HttpError::HeadTooLong => "HttpError::HeadTooLong".to_string(),
            HttpError::InvalidContentLength => "HttpError::InvalidContentLength".to_string(),
            HttpError::MalformedCookieHeader => "HttpError::MalformedCookieHeader".to_string(),
            HttpError::MalformedHeaderLine => "HttpError::MalformedHeaderLine".to_string(),
            HttpError::MalformedPath => "HttpError::MalformedPath".to_string(),
            HttpError::MalformedRequestLine => "HttpError::MalformedRequestLine".to_string(),
            HttpError::MissingRequestLine => "HttpError::MissingRequestLine".to_string(),
            HttpError::ResponseAlreadySent => "HttpError::ResponseAlreadySent".to_string(),
            HttpError::ResponseNotSent => "HttpError::ResponseNotSent".to_string(),
            HttpError::TimerThreadNotStarted => "HttpError::TimerThreadNotStarted".to_string(),
            HttpError::Truncated => "HttpError::Truncated".to_string(),
            HttpError::UnsupportedProtocol => "HttpError::UnsupportedProtocol".to_string(),
            HttpError::UnsupportedTransferEncoding => {
                "HttpError::UnsupportedTransferEncoding".to_string()
            }
            HttpError::UnwritableResponse => "HttpError::UnwritableResponse".to_string(),
        }
    }
}
impl From<HeadError> for HttpError {
    fn from(e: HeadError) -> Self {
        match e {
            HeadError::Truncated => HttpError::Truncated,
            HeadError::MissingRequestLine => HttpError::MissingRequestLine,
            HeadError::MalformedRequestLine => HttpError::MalformedRequestLine,
            HeadError::MalformedPath => HttpError::MalformedPath,
            HeadError::UnsupportedProtocol => HttpError::UnsupportedProtocol,
            HeadError::MalformedHeader => HttpError::MalformedHeaderLine,
        }
    }
}
impl From<HttpError> for Response {
    fn from(e: HttpError) -> Self {
        match e {
            HttpError::BodyNotUtf8
            | HttpError::InvalidContentLength
            | HttpError::MalformedCookieHeader
            | HttpError::MalformedHeaderLine
            | HttpError::MalformedPath
            | HttpError::MalformedRequestLine
            | HttpError::MissingRequestLine
            | HttpError::Truncated
            | HttpError::UnsupportedTransferEncoding => Response::text(400, e.description()),
            HttpError::Disconnected => Response::drop_connection(),
            HttpError::BodyTooLong => Response::text(413, "Uploaded data is too big."),
            HttpError::HeadTooLong => Response::text(431, e.description()),
            HttpError::UnsupportedProtocol => Response::text(505, e.description()),
            HttpError::AlreadyGotBody
            | HttpError::BodyNotAvailable
            | HttpError::BodyNotRead
            | HttpError::CacheDirNotConfigured
            | HttpError::DuplicateContentLengthHeader
            | HttpError::DuplicateContentTypeHeader
            | HttpError::DuplicateTransferEncodingHeader
            | HttpError::ErrorReadingFile(..)
            | HttpError::ErrorReadingResponseBody(..)
            | HttpError::ErrorSavingFile(..)
            | HttpError::HandlerDeadlineExceeded
            | HttpError::ResponseAlreadySent
            | HttpError::ResponseNotSent
            | HttpError::TimerThreadNotStarted
            | HttpError::UnwritableResponse => Response::text(500, "Internal server error"),
        }
    }
}
impl From<DeadlineExceededError> for HttpError {
    fn from(_: DeadlineExceededError) -> Self {
        HttpError::HandlerDeadlineExceeded
    }
}
impl From<DeadlineError> for HttpError {
    fn from(e: DeadlineError) -> Self {
        match e {
            DeadlineError::TimerThreadNotStarted => HttpError::TimerThreadNotStarted,
            DeadlineError::DeadlineExceeded => HttpError::HandlerDeadlineExceeded,
        }
    }
}
