use crate::internal::ToDateTime;
use crate::log::logger::{LogEvent, Logger};
use crate::log::prefix_file_set::{PrefixFile, PrefixFileSet};
use crate::log::{tag, Level};
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, SystemTime};

pub struct LogFile {
    pub file: File,
    pub len: u64,
    pub path: PathBuf,
    pub created: SystemTime,
}
impl LogFile {
    pub fn create(path_prefix: &Path) -> Result<Self, String> {
        for n in 0..u64::MAX {
            let dt = SystemTime::now().to_datetime();
            let mut path_str = path_prefix.as_os_str().to_os_string();
            path_str.push(format!(
                ".{:04}{:02}{:02}T{:02}{:02}{:02}Z-{n}",
                dt.year, dt.month, dt.day, dt.hour, dt.min, dt.sec
            ));
            let path: PathBuf = PathBuf::from(path_str);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        file,
                        len: 0,
                        path,
                        created: SystemTime::now(),
                    })
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
                Err(e) => {
                    return Err(format!("error creating file {path:?}: {e:?}"));
                }
            };
        }
        unreachable!();
    }

    pub fn write_all(&mut self, buffer: &Vec<u8>) -> Result<(), String> {
        self.file
            .write_all(buffer)
            .map_err(|e| format!("error writing file {:?}: {e:?}", self.path))?;
        self.len += buffer.len() as u64;
        Ok(())
    }

    pub fn age(&self, now: SystemTime) -> Duration {
        now.duration_since(self.created)
            .unwrap_or(Duration::from_secs(0))
    }
}

pub struct LogFileWriterBuilder {
    pub max_keep_age: Option<Duration>,
    pub max_keep_bytes: u64,
    pub max_write_age: Duration,
    pub max_write_bytes: u64,
    pub path_prefix: PathBuf,
}
impl LogFileWriterBuilder {
    /// Configures the struct to continuously delete files that match `path_prefix`
    /// and are older than `max_keep_age`.
    ///
    /// Defaults to 24 hours.
    ///
    /// # Panics
    /// Panics when `duration` is less than 1 minute.
    pub fn with_max_keep_age(mut self, duration: Duration) -> Self {
        if duration < Duration::from_secs(60) {
            panic!("duration is less than 1 minute: {duration:?}");
        }
        self.max_keep_age = Some(duration);
        self
    }

    /// Configures the struct to close the current file and create a new file whenever
    /// the current file is older than `duration`.
    ///
    /// Defaults to 10 MiB.
    ///
    /// # Panics
    /// Panics when `duration` is less than 1 second.
    pub fn with_max_write_age(mut self, duration: Duration) -> Self {
        if duration < Duration::from_secs(1) {
            panic!("duration is less than 1 second: {duration:?}");
        }
        self.max_write_age = duration;
        self
    }

    /// Configures the struct to write `len` bytes or less to each log file
    /// before switching to a new one.
    ///
    /// # Panics
    /// Panics when `len` is less than 64 KiB.
    pub fn with_max_write_bytes(mut self, len: u64) -> Self {
        if len < (64 * 1024) {
            panic!("len is less than 64 KiB: {len}");
        }
        self.max_write_bytes = len;
        self
    }

    pub fn start_writer_thread(self) -> Result<LogFileWriter, String> {
        LogFileWriter::start_writer_thread(self)
    }
}

#[derive(Clone)]
pub struct LogFileWriter(Sender<LogEvent>);
impl LogFileWriter {
    /// Creates a new struct to write log files.
    /// It writes files with names that start with `path_prefix`.
    ///
    /// DATA LOSS WARNING: Automatically deletes files that match
    /// `path_prefix` so the total size of the files is `max_keep_bytes`.
    /// Deletes the oldest files.
    ///
    /// # Panics
    /// Panics when `path_prefix` does not end with a filename part.
    ///
    /// # Example
    /// ```no_run
    /// use std::path::PathBuf;
    /// use servlin::log::LogFileWriter;
    /// let writer = LogFileWriter::new_builder(
    ///     PathBuf::from("/var/log/server.log"),
    ///     100 * 1024 * 1024,
    /// ).build();
    /// ```
    pub fn new_builder(
        path_prefix: impl Into<PathBuf>,
        max_keep_bytes: u64,
    ) -> LogFileWriterBuilder {
        let path_prefix = path_prefix.into().canonicalize().unwrap();
        if path_prefix.file_name().is_none() {
            panic!(
                "path_prefix does not contain a filename part: {:?}",
                path_prefix.to_string_lossy()
            );
        }
        LogFileWriterBuilder {
            max_keep_age: None,
            max_keep_bytes,
            max_write_age: Duration::from_secs(24 * 3600),
            max_write_bytes: 10 * 1024 * 1024,
            path_prefix,
        }
    }

    pub fn start_writer_thread(builder: LogFileWriterBuilder) -> Result<Self, String> {
        let mut file_set = PrefixFileSet::new(&builder.path_prefix)?;
        file_set.delete_oldest_while_over_max_len(builder.max_keep_bytes)?;
        let mut file = LogFile::create(&builder.path_prefix)?;
        let mut buffer: Vec<u8> = Vec::new();
        LogEvent::new(Level::Info, tag("msg", "Starting log writer"))
            .write_json(&mut buffer)
            .unwrap();
        file.write_all(&buffer)?;
        buffer.clear();
        let (sender, receiver): (Sender<LogEvent>, Receiver<LogEvent>) = channel();
        std::thread::spawn(move || {
            for event in receiver {
                event.write_json(&mut buffer).unwrap();
                let now = SystemTime::now();
                if file.len + (buffer.len() as u64) > builder.max_write_bytes
                    || file.age(now) > builder.max_write_age
                {
                    file_set.push(PrefixFile {
                        path: file.path.clone(),
                        mtime: now,
                        len: file.len,
                    });
                    file = LogFile::create(&builder.path_prefix).unwrap();
                }
                if let Some(duration) = builder.max_keep_age {
                    file_set.delete_older_than(now, duration).unwrap();
                }
                file_set
                    .delete_oldest_while_over_max_len(
                        builder.max_keep_bytes - file.len - (buffer.len() as u64),
                    )
                    .unwrap();
                file.write_all(&buffer).unwrap();
                buffer.clear();
            }
        });
        Ok(LogFileWriter(sender))
    }
}
impl Logger for LogFileWriter {
    fn add(&self, event: LogEvent) {
        self.0.send(event).unwrap()
    }

    fn rc_clone(&self) -> Rc<dyn Logger> {
        Rc::new(self.clone())
    }
}
