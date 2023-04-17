use crate::internal::{EpochNs, FormatTime, ToDateTime};
use crate::log::tag::Tag;
use crate::log::tag_list::TagList;
use crate::log::tag_value::TagValue;
use crate::log::Level;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::Mutex;
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
            write!(f, "{{\"time_ns\":{time_ns},\"time\":\"{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z\",\"level\":\"{level}\"}}")
        } else {
            write!(f, "{{\"time_ns\":{time_ns},\"time\":\"{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z\",\"level\":\"{level}\",{tags}}}")
        }
    }
}

pub trait Logger: Send {
    fn add(&self, event: LogEvent);
    fn rc_clone(&self) -> Rc<dyn Logger>;
}

#[derive(Clone)]
pub struct StdoutLogger {}
impl Logger for StdoutLogger {
    fn add(&self, event: LogEvent) {
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

    fn rc_clone(&self) -> Rc<dyn Logger> {
        Rc::new(self.clone())
    }
}

pub static GLOBAL_LOGGER: once_cell::sync::OnceCell<Mutex<Box<dyn Logger>>> =
    once_cell::sync::OnceCell::new();

thread_local! {
    pub static THREAD_LOCAL_LOGGER: Option<Rc<dyn Logger>> = None;
    pub static THREAD_LOCAL_TAGS: RefCell<Vec<Tag>> = RefCell::new(Vec::new());
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GlobalLoggerAlreadySetError {}

pub fn set_global_logger(logger: impl Logger + 'static) -> Result<(), GlobalLoggerAlreadySetError> {
    GLOBAL_LOGGER
        .set(Mutex::new(Box::new(logger)))
        .map_err(|_| GlobalLoggerAlreadySetError {})
}

pub static STDOUT_LOGGER: StdoutLogger = StdoutLogger {};

/// Gets the logger previously passed to [set_global_logger].
/// Returns [StdoutLogger] if no global logger was set.
pub fn global_logger() -> &'static (dyn Logger) {
    if let Some(box_logger) = THREAD_LOCAL_LOGGER.with(|opt_rc| opt_rc.map(|rc| rc.clone())) {
        box_logger.as_ref()
    } else if let Some(mutex_box_logger) = GLOBAL_LOGGER.get() {
        let box_logger = mutex_box_logger.lock().unwrap().rc_clone();
        THREAD_LOCAL_LOGGER.with(|cell| {
            cell.set(box_logger);
            cell.get().unwrap().as_ref()
        })
    } else {
        &STDOUT_LOGGER
    }
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

pub fn log(time: SystemTime, level: Level, tags: impl Into<TagList>) {
    let mut tags = tags.into();
    with_thread_local_log_tags(|thread_tags| tags.0.extend_from_slice(thread_tags));
    let event = LogEvent { time, level, tags };
    global_logger().add(event);
}
