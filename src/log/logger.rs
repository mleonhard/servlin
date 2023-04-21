use crate::internal::{EpochTime, FormatTime, ToDateTime};
use crate::log::tag::Tag;
use crate::log::tag_list::TagList;
use crate::log::tag_value::TagValue;
use crate::log::Level;
use crate::Request;
use std::cell::RefCell;
use std::io::Write;
use std::ops::Deref;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Mutex, MutexGuard};
use std::time::SystemTime;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LogEvent {
    time: SystemTime,
    level: Level,
    tags: TagList,
}
impl LogEvent {
    pub fn new(level: Level, tags: impl Into<TagList>) -> Self {
        Self {
            time: SystemTime::now(),
            level,
            tags: tags.into(),
        }
    }

    /// Writes the log event to to `f` in JSONL format.
    ///
    /// # Errors
    /// Returns `Err` when it fails to write to `f`.
    pub fn write_json(&self, f: &mut impl Write) -> Result<(), std::io::Error> {
        // "time_ns":1681457536082810000,"time":"2023-04-14T00:32:16.082-07:00"
        let time_ns = self.time.epoch_ns();
        let dt = self.time.to_datetime();
        let year = dt.year;
        let month = dt.month;
        let day = dt.day;
        let hour = dt.hour;
        let min = dt.min;
        let sec = dt.sec;
        let level = self.level;
        let tags = &self.tags;
        if tags.is_empty() {
            writeln!(f, "{{\"time\":\"{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z\",\"level\":\"{level}\",\"time_ns\":{time_ns}}}")
        } else {
            writeln!(f, "{{\"time\":\"{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z\",\"level\":\"{level}\",{tags},\"time_ns\":{time_ns}}}")
        }
    }
}

#[must_use]
pub fn start_stdout_logger_thread() -> SyncSender<LogEvent> {
    let (sender, receiver): (SyncSender<LogEvent>, Receiver<LogEvent>) = sync_channel(100);
    std::thread::spawn(move || {
        for event in receiver {
            let time = event.time.iso8601_utc();
            let level = event.level;
            let mut tags = event.tags;
            if let Some(msg_index) = tags.iter().position(|tag| tag.name == "msg") {
                let msg_tag = tags.remove(msg_index);
                let msg = msg_tag.value;
                println!("{time} {level} {msg} {tags}");
            } else {
                println!("{time} {level} {tags}");
            }
        }
    });
    sender
}

#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn start_stdout_jsonl_logger_thread() -> SyncSender<LogEvent> {
    let (sender, receiver): (SyncSender<LogEvent>, Receiver<LogEvent>) = sync_channel(100);
    std::thread::spawn(move || {
        let mut stdout = std::io::stdout();
        for event in receiver {
            event.write_json(&mut stdout).unwrap();
        }
    });
    sender
}

#[derive(Debug)]
pub enum GlobalLoggerState {
    None,
    Some(SyncSender<LogEvent>),
    Default(SyncSender<LogEvent>),
}
impl GlobalLoggerState {
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, GlobalLoggerState::None)
    }

    #[must_use]
    pub fn is_some(&self) -> bool {
        matches!(self, GlobalLoggerState::Some(..))
    }
}

pub static GLOBAL_LOGGER: once_cell::sync::OnceCell<Mutex<GlobalLoggerState>> =
    once_cell::sync::OnceCell::new();

thread_local! {
    pub static THREAD_LOCAL_TAGS: RefCell<Vec<Tag>> = RefCell::new(Vec::new());
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GlobalLoggerAlreadySetError {}

#[allow(clippy::module_name_repetitions)]
pub fn lock_global_logger() -> MutexGuard<'static, GlobalLoggerState> {
    GLOBAL_LOGGER
        .get_or_init(|| Mutex::new(GlobalLoggerState::None))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClearGlobalLoggerOnDrop {}
impl Drop for ClearGlobalLoggerOnDrop {
    fn drop(&mut self) {
        let mut guard = lock_global_logger();
        assert!(guard.is_some());
        *guard = GlobalLoggerState::None;
    }
}

/// Sets the global logger.
/// Returns a [`ClearGlobalLoggerOnDrop`] which clears the global logger when it drops.
///
/// # Errors
/// Returns `Err` when a global logger is already set.
#[allow(clippy::module_name_repetitions)]
pub fn set_global_logger(
    sender: SyncSender<LogEvent>,
) -> Result<ClearGlobalLoggerOnDrop, GlobalLoggerAlreadySetError> {
    let mut mutex_guard = lock_global_logger();
    if mutex_guard.is_some() {
        return Err(GlobalLoggerAlreadySetError {});
    }
    *mutex_guard = GlobalLoggerState::Some(sender);
    Ok(ClearGlobalLoggerOnDrop {})
}

pub struct GlobalLoggerGuard {
    mutex_guard: MutexGuard<'static, GlobalLoggerState>,
}
impl GlobalLoggerGuard {
    #[must_use]
    pub fn new(mutex_guard: MutexGuard<'static, GlobalLoggerState>) -> Option<Self> {
        if mutex_guard.is_none() {
            None
        } else {
            Some(Self { mutex_guard })
        }
    }
}
impl Deref for GlobalLoggerGuard {
    type Target = SyncSender<LogEvent>;

    fn deref(&self) -> &Self::Target {
        match &*self.mutex_guard {
            GlobalLoggerState::None => unreachable!(),
            GlobalLoggerState::Some(sender) | GlobalLoggerState::Default(sender) => sender,
        }
    }
}

/// Gets the logger previously passed to [`set_global_logger`].
/// When no global logger has been set, starts a default [`StdoutLogger`] and returns it.
/// Calling [`set_global_logger`] later will replace this default logger.
#[allow(clippy::missing_panics_doc)]
#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn global_logger() -> GlobalLoggerGuard {
    let mut mutex_guard = lock_global_logger();
    if mutex_guard.is_none() {
        *mutex_guard = GlobalLoggerState::Default(start_stdout_logger_thread());
    }
    GlobalLoggerGuard::new(mutex_guard).unwrap()
}

pub fn add_thread_local_log_tag(name: &'static str, value: impl Into<TagValue>) {
    let tag = Tag::new(name, value);
    THREAD_LOCAL_TAGS.with(|cell| cell.borrow_mut().push(tag));
}

pub fn clear_thread_local_log_tags() {
    THREAD_LOCAL_TAGS.with(|cell| cell.borrow_mut().clear());
}

pub fn with_thread_local_log_tags<R, F: FnOnce(&[Tag]) -> R>(f: F) -> R {
    THREAD_LOCAL_TAGS.with(|cell| f(cell.borrow().as_slice()))
}

pub fn add_thread_local_log_tags_from_request(req: &Request) {
    add_thread_local_log_tag("http_method", req.method());
    add_thread_local_log_tag("path", req.url().path());
    add_thread_local_log_tag("request_id", req.id);
    if let Some(len) = req.body.len() {
        add_thread_local_log_tag("request_body_len", len);
    } else {
        add_thread_local_log_tag("request_body", "pending");
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct LoggerStoppedError {}

/// Make a new log event and sends it to the global logger.
///
/// Logs to stdout when no global logger is set.
///
/// # Errors
/// Returns `Err` when the global logger has stopped.
pub fn log(
    time: SystemTime,
    level: Level,
    tags: impl Into<TagList>,
) -> Result<(), LoggerStoppedError> {
    let mut tags = tags.into();
    with_thread_local_log_tags(|thread_tags| tags.0.extend_from_slice(thread_tags));
    let event = LogEvent { time, level, tags };
    global_logger()
        .send(event)
        .map_err(|_| LoggerStoppedError {})
}
