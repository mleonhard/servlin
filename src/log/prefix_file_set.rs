use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::remove_file;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub struct PrefixFile {
    pub path: PathBuf,
    pub mtime: SystemTime,
    pub len: u64,
}
impl PartialEq for PrefixFile {
    fn eq(&self, other: &Self) -> bool {
        other.mtime.eq(&self.mtime)
    }
}
impl Eq for PrefixFile {}
impl PartialOrd for PrefixFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.mtime.partial_cmp(&self.mtime)
    }
}
impl Ord for PrefixFile {
    fn cmp(&self, other: &Self) -> Ordering {
        other.mtime.cmp(&self.mtime)
    }
}

pub struct PrefixFileSet {
    files: BinaryHeap<PrefixFile>,
    len: u64,
}
impl PrefixFileSet {
    pub fn new(path_prefix: &Path) -> Result<Self, String> {
        let dir = path_prefix
            .parent()
            .ok_or_else(|| format!("path has no parent: {path_prefix:?}"))?;
        let mut files = BinaryHeap::new();
        for dir_entry in dir
            .read_dir()
            .map_err(|e| format!("error reading dir {dir:?}: {e:?}"))?
        {
            let dir_entry = dir_entry.map_err(|e| format!("error reading dir {dir:?}: {e:?}"))?;
            let path = dir_entry.path();
            if path.starts_with(&path_prefix) {
                let metadata = dir_entry.metadata().map_err(|e| {
                    format!("error reading metadata of {:?}: {e:?}", dir_entry.path())
                })?;
                if metadata.is_file() {
                    let mtime = metadata.modified().unwrap();
                    let len = metadata.len();
                    files.push(PrefixFile { path, mtime, len })
                }
            }
        }
        let len = files.iter().map(|f| f.len).sum();
        Ok(Self { files, len })
    }

    pub fn delete_oldest(&mut self) -> Result<(), String> {
        let file = self.files.peek().unwrap();
        remove_file(&file.path)
            .map_err(|e| format!("error deleting file {:?}: {e:?}", file.path))?;
        self.len -= file.len;
        self.files.pop();
        Ok(())
    }

    pub fn delete_older_than(&mut self, now: SystemTime, duration: Duration) -> Result<(), String> {
        let min_mtime = now - duration;
        while let Some(file) = self.files.peek() {
            if file.mtime < min_mtime {
                self.delete_oldest()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    pub fn delete_oldest_while_over_max_len(&mut self, max_len: u64) -> Result<(), String> {
        while self.len > max_len {
            self.delete_oldest()?;
        }
        Ok(())
    }

    pub fn push(&mut self, file: PrefixFile) {
        self.files.push(file);
    }
}
