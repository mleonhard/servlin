use crate::internal::ToDateTime;
use crate::log::logger::LogEvent;
use crate::log::prefix_file_set::{PrefixFile, PrefixFileSet};
use crate::log::{tag, Level};
use crate::Error;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::time::{Duration, SystemTime};

pub struct LogFile {
    pub file: File,
    pub len: u64,
    pub path: PathBuf,
    pub created: SystemTime,
}
impl LogFile {
    /// Creates a new file on disk with a name that starts with `path_prefix`.
    /// If files already exist, it tries other filename suffixes
    /// until it finds one that does not exist.
    ///
    /// # Errors
    /// Returns `Err` when it fails to create the file.
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

    /// Writes all of the bytes in `buffer` to the file.
    ///
    /// # Errors
    /// Returns `Err` when it fails to write to the file.
    pub fn write_all(&mut self, buffer: &Vec<u8>) -> Result<(), String> {
        self.file
            .write_all(buffer)
            .map_err(|e| format!("error writing file {:?}: {e:?}", self.path))?;
        self.len += buffer.len() as u64;
        Ok(())
    }

    /// Returns the duration between the file's creation and `now`.
    #[must_use]
    pub fn age(&self, now: SystemTime) -> Duration {
        now.duration_since(self.created)
            .unwrap_or(Duration::from_secs(0))
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct LogFileWriter {
    pub max_keep_age: Option<Duration>,
    pub max_keep_bytes: u64,
    pub max_write_age: Duration,
    pub max_write_bytes: u64,
    pub path_prefix: PathBuf,
}
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
    /// ).start_writer_thread();
    /// ```
    #[must_use]
    pub fn new_builder(path_prefix: impl Into<PathBuf>, max_keep_bytes: u64) -> LogFileWriter {
        LogFileWriter {
            max_keep_age: None,
            max_keep_bytes,
            max_write_age: Duration::from_secs(24 * 3600),
            max_write_bytes: 10 * 1024 * 1024,
            path_prefix: path_prefix.into(),
        }
    }

    /// Configures the struct to continuously delete files that match `path_prefix`
    /// and are older than `max_keep_age`.
    ///
    /// Defaults to 24 hours.
    ///
    /// # Panics
    /// Panics when `duration` is less than 1 minute.
    #[must_use]
    pub fn with_max_keep_age(mut self, duration: Duration) -> Self {
        assert!(
            duration >= Duration::from_secs(60),
            "duration is less than 1 minute: {duration:?}"
        );
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
    #[must_use]
    pub fn with_max_write_age(mut self, duration: Duration) -> Self {
        assert!(
            duration >= Duration::from_secs(1),
            "duration is less than 1 second: {duration:?}"
        );
        self.max_write_age = duration;
        self
    }

    /// Configures the struct to write `len` bytes or less to each log file
    /// before switching to a new one.
    ///
    /// # Panics
    /// Panics when `len` is less than 64 KiB.
    #[must_use]
    pub fn with_max_write_bytes(mut self, len: u64) -> Self {
        assert!(len >= (64 * 1024), "len is less than 64 KiB: {len}");
        self.max_write_bytes = len;
        self
    }

    /// Creates the first log file and starts the log file writer thread.
    ///
    /// # Errors
    /// Returns `Err` when it fails to create the first log file.
    #[allow(clippy::missing_panics_doc)]
    pub fn start_writer_thread(self) -> Result<SyncSender<LogEvent>, Error> {
        let dir = self
            .path_prefix
            .parent()
            .ok_or_else(|| format!("path has no parent: {:?}", self.path_prefix))?;
        let mut path_prefix = if dir.is_absolute() {
            dir.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| format!("cannot find current directory to resolve path {dir:?}: {e}"))?
                .join(dir)
        }
        .canonicalize()
        .map_err(|e| {
            format!(
                "error getting canonical path of {:?}: {e}",
                self.path_prefix
            )
        })?;
        let file_name = self.path_prefix.file_name().ok_or_else(|| {
            format!(
                "path_prefix does not contain a filename part: {:?}",
                path_prefix.to_string_lossy()
            )
        })?;
        path_prefix.push(Path::new(file_name));
        let mut file_set = PrefixFileSet::new(&path_prefix)?;
        file_set.delete_oldest_while_over_max_len(self.max_keep_bytes)?;
        let mut file = LogFile::create(&path_prefix)?;
        let mut buffer: Vec<u8> = Vec::new();
        LogEvent::new(Level::Info, tag("msg", "Starting log writer"))
            .write_jsonl(&mut buffer)
            .unwrap();
        file.write_all(&buffer)?;
        buffer.clear();
        let (sender, receiver): (SyncSender<LogEvent>, Receiver<LogEvent>) = sync_channel(100);
        std::thread::spawn(move || {
            for event in receiver {
                event.write_jsonl(&mut buffer).unwrap();
                let now = SystemTime::now();
                if file.len + (buffer.len() as u64) > self.max_write_bytes
                    || file.age(now) > self.max_write_age
                {
                    file_set.push(PrefixFile {
                        path: file.path.clone(),
                        mtime: now,
                        len: file.len,
                    });
                    file = LogFile::create(&path_prefix).unwrap();
                }
                if let Some(duration) = self.max_keep_age {
                    file_set.delete_older_than(now, duration).unwrap();
                }
                file_set
                    .delete_oldest_while_over_max_len(
                        self.max_keep_bytes - file.len - (buffer.len() as u64),
                    )
                    .unwrap();
                file.write_all(&buffer).unwrap();
                buffer.clear();
            }
            file.file.sync_all().unwrap();
        });
        Ok(sender)
    }
}
