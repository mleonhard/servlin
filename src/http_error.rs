use crate::head::HeadError;
use crate::Response;
use std::io::ErrorKind;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub enum HttpError {
    BodyNotAvailable,
    BodyNotRead,
    BodyNotUtf8,
    BodyTooLong,
    Disconnected,
    ErrorReadingFile(ErrorKind, String),
    ErrorSavingFile(ErrorKind, String),
    HeadTooLong,
    InvalidContentLength,
    MalformedHeaderLine,
    MalformedPath,
    MalformedRequestLine,
    MissingRequestLine,
    ResponseAlreadySent,
    ResponseNotSent,
    Truncated,
    UnsupportedProtocol,
    UnsupportedTransferEncoding,
    UnwritableResponse,
}
impl HttpError {
    #[must_use]
    pub fn is_server_error(&self) -> bool {
        match self {
            HttpError::BodyNotAvailable
            | HttpError::BodyNotRead
            | HttpError::ErrorReadingFile(_, _)
            | HttpError::ErrorSavingFile(_, _)
            | HttpError::ResponseAlreadySent
            | HttpError::ResponseNotSent
            | HttpError::UnwritableResponse => true,
            HttpError::BodyNotUtf8
            | HttpError::BodyTooLong
            | HttpError::Disconnected
            | HttpError::HeadTooLong
            | HttpError::InvalidContentLength
            | HttpError::MalformedHeaderLine
            | HttpError::MalformedPath
            | HttpError::MalformedRequestLine
            | HttpError::MissingRequestLine
            | HttpError::Truncated
            | HttpError::UnsupportedProtocol
            | HttpError::UnsupportedTransferEncoding => false,
        }
    }

    #[must_use]
    pub fn description(&self) -> String {
        match self {
            HttpError::BodyNotAvailable => "HttpError::BodyNotAvailable".to_string(),
            HttpError::BodyNotRead => "HttpError::BodyNotRead".to_string(),
            HttpError::BodyNotUtf8 => "HttpError::BodyNotUtf8".to_string(),
            HttpError::BodyTooLong => "HttpError::BodyTooLong".to_string(),
            HttpError::Disconnected => "HttpError::Disconnected".to_string(),
            HttpError::ErrorReadingFile(kind, s) | HttpError::ErrorSavingFile(kind, s) => {
                format!("{:?}: {}", kind, s)
            }
            HttpError::HeadTooLong => "HttpError::HeadTooLong".to_string(),
            HttpError::InvalidContentLength => "HttpError::InvalidContentLength".to_string(),
            HttpError::MalformedHeaderLine => "HttpError::MalformedHeaderLine".to_string(),
            HttpError::MalformedPath => "HttpError::MalformedPath".to_string(),
            HttpError::MalformedRequestLine => "HttpError::MalformedRequestLine".to_string(),
            HttpError::MissingRequestLine => "HttpError::MissingRequestLine".to_string(),
            HttpError::ResponseAlreadySent => "HttpError::ResponseAlreadySent".to_string(),
            HttpError::ResponseNotSent => "HttpError::ResponseNotSent".to_string(),
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
            | HttpError::MalformedHeaderLine
            | HttpError::MalformedPath
            | HttpError::MalformedRequestLine
            | HttpError::MissingRequestLine
            | HttpError::Truncated
            | HttpError::UnsupportedTransferEncoding => Response::text(400, e.description()),
            HttpError::Disconnected => Response::Drop,
            HttpError::BodyTooLong => Response::text(413, "Uploaded data is too big."),
            HttpError::HeadTooLong => Response::text(431, e.description()),
            HttpError::UnsupportedProtocol => Response::text(505, e.description()),
            HttpError::BodyNotAvailable
            | HttpError::BodyNotRead
            | HttpError::ErrorReadingFile(..)
            | HttpError::ErrorSavingFile(..)
            | HttpError::ResponseAlreadySent
            | HttpError::ResponseNotSent
            | HttpError::UnwritableResponse => Response::text(500, "Internal server error"),
        }
    }
}
impl From<HttpError> for std::io::Error {
    fn from(e: HttpError) -> Self {
        match e {
            HttpError::Truncated => {
                std::io::Error::new(ErrorKind::UnexpectedEof, "Incomplete request")
            }
            HttpError::BodyNotUtf8
            | HttpError::BodyTooLong
            | HttpError::HeadTooLong
            | HttpError::InvalidContentLength
            | HttpError::MalformedHeaderLine
            | HttpError::MalformedPath
            | HttpError::MalformedRequestLine
            | HttpError::MissingRequestLine
            | HttpError::UnsupportedProtocol
            | HttpError::UnsupportedTransferEncoding => {
                std::io::Error::new(ErrorKind::InvalidData, e.description())
            }
            HttpError::Disconnected => {
                std::io::Error::new(ErrorKind::ConnectionReset, e.description())
            }
            HttpError::ErrorReadingFile(kind, s) | HttpError::ErrorSavingFile(kind, s) => {
                std::io::Error::new(kind, s)
            }
            HttpError::BodyNotAvailable
            | HttpError::BodyNotRead
            | HttpError::ResponseAlreadySent
            | HttpError::ResponseNotSent
            | HttpError::UnwritableResponse => {
                std::io::Error::new(ErrorKind::InvalidInput, e.description())
            }
        }
    }
}
