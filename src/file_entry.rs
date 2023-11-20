use serde::{Serialize, Deserialize};
use std::{time::SystemTime, cmp::Ordering};
use anyhow::{anyhow, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub len: u64,
    pub modified: SystemTime,
    pub tar_position_after: Option<usize>,
}

impl PartialEq for FileEntry {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
impl Eq for FileEntry {}

impl Ord for FileEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for FileEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FileEntry {
    pub fn new(path: &str) -> Result<Self> {
        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() {
            return Err(anyhow!(format!("{path} is not a file")));
        }

        let ret = Self {
            path: path.to_string(),
            modified: metadata.modified().map_err(|e|anyhow!(e))?,
            len: metadata.len(),
            tar_position_after: None,
        };
        Ok(ret)
    }

    pub fn key(&self) -> String {
        format!("{}|{:?}|{}",self.len,self.modified,self.path)
    }
}
