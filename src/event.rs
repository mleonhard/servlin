use crate::util::escape_and_elide;
use futures_lite::FutureExt;
use safina_sync::SyncSender;
use std::io::{Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub enum Event {
    /// Message(data)
    Message(String),
    /// Custom(event_type, data)<br>
    /// Use [`Event::custom`] to make this variant.
    Custom(String, String),
}
impl Event {
    /// # Errors
    /// Returns an error when `event` contains newlines.
    pub fn custom(event_type: impl AsRef<str>, data: String) -> Result<Self, String> {
        if event_type.as_ref().contains('\r') || event_type.as_ref().contains('\n') {
            Err(format!(
                "Event::message called with `event_type` containing newlines: {}",
                escape_and_elide(event_type.as_ref().as_bytes(), 100)
            ))
        } else {
            Ok(Self::Custom(event_type.as_ref().to_string(), data))
        }
    }

    /// # Errors
    /// Returns `ErrorKind::WriteZero` when `buf` is not big enough to hold the event data.
    #[allow(clippy::write_with_newline)]
    pub fn write_to(&self, mut buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let original_buf_len = buf.len();
        let data = match self {
            Event::Message(data) => data,
            Event::Custom(event_type, data) => {
                write!(buf, "event: {}\n", event_type)?;
                data
            }
        };
        for line in data.lines() {
            write!(buf, "data: {}\n", line)?;
        }
        Ok(original_buf_len - buf.len())
    }

    #[allow(clippy::write_with_newline)]
    pub fn push_to(&self, buf: &mut Vec<u8>) {
        let data = match self {
            Event::Message(data) => data,
            Event::Custom(event_type, data) => {
                write!(buf, "event: {}\n", event_type).unwrap();
                data
            }
        };
        for line in data.lines() {
            write!(buf, "data: {}\n", line).unwrap();
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSender(pub Option<SyncSender<Event>>);
impl EventSender {
    pub fn send(&mut self, event: Event) {
        if let Some(sender) = &self.0 {
            if sender.try_send(event).is_err() {
                self.0.take();
            }
        }
    }
}

// TODO: Support reading messages that are larger than `buf`.
#[allow(clippy::module_name_repetitions)]
pub struct EventReceiver(pub safina_sync::Receiver<Event>);
impl futures_io::AsyncRead for EventReceiver {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(event)) => Poll::Ready(event.write_to(buf)),
            Poll::Ready(Err(_recv_error)) => Poll::Ready(Ok(0)),
        }
    }
}
impl Read for EventReceiver {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        match self.0.recv() {
            Ok(event) => event.write_to(buf),
            Err(_) => Ok(0),
        }
    }
}
