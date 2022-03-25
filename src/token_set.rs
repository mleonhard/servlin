#![allow(dead_code)]
use safina_sync::{sync_channel, Receiver, SyncSender};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

/// A token.  If the token came from a `TokenSet`, dropping the token puts it back in the set.
pub struct Token(SyncSender<()>);
impl Token {
    /// Makes a new token that is not part of a set.  This is useful for testing.
    #[must_use]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let (sender, _receiver) = sync_channel(1);
        Self(sender)
    }
}
impl Drop for Token {
    fn drop(&mut self) {
        let _ = self.0.try_send(());
    }
}

pub struct TimeOut;

/// A set of tokens.  You can get a token from the set.
/// Dropping the token returns it to the set.
/// When the set is empty, you must wait for a token to be returned.
///
/// This struct is useful for limiting the number of things that can happen at the same time.
/// For example, you can use it to limit the number of connections a server handles.
pub struct TokenSet(SyncSender<()>, Receiver<()>);
impl TokenSet {
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn new(size: usize) -> Self {
        let (sender, receiver) = safina_sync::sync_channel(size);
        for _ in 0..size {
            sender.try_send(()).unwrap();
        }
        Self(sender, receiver)
    }

    #[allow(clippy::missing_panics_doc)]
    pub async fn async_wait_token(&mut self) -> Token {
        self.1.async_recv().await.unwrap();
        Token(self.0.clone())
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn wait_token(&self) -> Token {
        self.1.recv().unwrap();
        Token(self.0.clone())
    }

    /// # Errors
    /// Returns an error when `timeout` passes and it has not obtained a token.
    pub fn wait_token_timeout(&self, timeout: Duration) -> Result<Token, TimeOut> {
        match self.1.recv_timeout(timeout) {
            Ok(_) => Ok(Token(self.0.clone())),
            Err(RecvTimeoutError::Timeout) => Err(TimeOut),
            Err(RecvTimeoutError::Disconnected) => unreachable!(),
        }
    }
}
