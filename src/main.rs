use clap::Parser;
use log::info;
use serde::{Serialize, Deserialize};
use std::{time::SystemTime, fs::{self, File}, io::{self, Write}, cmp::Ordering, env};
use anyhow::{anyhow, Result};
use serde_json::json;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use tar::Builder;

const OVERHEAD_ESTIMATE_BYTES_PER_FILE: usize = 1024;

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


#[derive(Debug)]
pub struct TarOutput {
    output_file: File,
    offset: usize,
    position: usize,
}

impl TarOutput {
    pub fn new(path: &str, offset: usize) -> Self {
        Self {
            output_file: File::create(path).unwrap(),
            offset,
            position: 0,
        }
        
    }

    pub fn is_earlier_than(&self, file_size: usize) -> bool {
        self.position+OVERHEAD_ESTIMATE_BYTES_PER_FILE+file_size < self.offset
    }
}

impl Write for TarOutput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();
        if self.position+len < self.offset {
            self.position += len;
            return Ok(len);
        }
        if self.position < self.offset {
            let diff = self.offset - self.position-1;
            self.output_file.write(&buf[diff..])?;
            self.position += len;
            return Ok(len);
        }
        self.position += len;
        self.output_file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output_file.flush()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FileList {
    output_file: Option<String>,
    meta_file: Option<String>,
    entries: Vec<FileEntry>,
    offset: Option<usize>,
}

impl FileList {
    pub fn set_output_file(&mut self, filename: &str) {
        self.output_file = Some(filename.to_string());
    }

    pub fn set_files(&mut self, files: &Vec<String>) -> Result<()> {
        let current_entries = self.get_current_entries(files)?;
        let meta_file = self.meta_file(&current_entries);
        self.entries = match fs::metadata(&meta_file) {
            Ok(_) => {
                let s = fs::read_to_string(&meta_file).unwrap();
                let stored_entries = serde_json::from_str(&s).unwrap();
                self.validate_metadata(&stored_entries,&current_entries)?;
                stored_entries
            },
            Err(_) => {
                current_entries
            },
        };
        Ok(())
    }

    pub fn output(&mut self) -> Result<()>{
        let mut tar_builder = self.tar_builder()?;
        for entry_id in 0..self.entries.len() {
            let entry = self.entries[entry_id].to_owned();
            let position_after = entry.tar_position_after;
            if tar_builder.get_ref().is_earlier_than(entry.len as usize) && position_after.is_some() {
                info!("Skipping {}",entry.path);
                tar_builder.get_mut().position = position_after.unwrap(); // Safe
            } else {
                // Output file
                tar_builder.append_path(&entry.path)?;
                self.entries[entry_id].tar_position_after = Some(tar_builder.get_ref().position);
                self.write_meta_file()?;
            }
        }
        Ok(())
    }

    fn get_current_entries(&self, files: &Vec<String>) -> Result<Vec<FileEntry>> {
        let mut ret: Vec<FileEntry> = files.iter()
            .filter_map(|filename| FileEntry::new(filename).ok())
            .collect();
        ret.sort();
        ret.dedup();
        Ok(ret)
    }

    fn get_sha256(&self, text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text);
        let bytes = &hasher.finalize()[..];
        general_purpose::STANDARD.encode(bytes)
    }

    fn meta_file(&mut self, entries: &Vec<FileEntry>) -> String {
        match &self.meta_file {
            Some(mf) => mf.to_owned(),
            None => {
                let key = entries.iter().map(|e| e.key()).collect::<Vec<String>>().join("\n");
                let base_64 = self.get_sha256(&key).replace("=","").replace("/","").replace("+","").to_uppercase();
                let tmp_dir = env::temp_dir();
                let filename = format!("{}{base_64}.ziplive",tmp_dir.display());
                info!("Using meta file {filename}");
                self.meta_file = Some(filename.to_owned());
                filename
            }
        }
    }

    fn validate_metadata(&mut self,stored: &Vec<FileEntry>, current: &Vec<FileEntry>) -> Result<()> {
        let meta_file = self.meta_file(current);
        if stored.len()!=current.len() {
            return Err(anyhow!(format!("Metadata data file {meta_file} has {} files but {} files were given",stored.len(),current.len())));
        }
        let lists_differ = stored.iter()
            .zip(current.iter())
            .any(|(s,c)| s.len!=c.len || s.modified!=c.modified);
        if lists_differ {
            return Err(anyhow!("Files have changed since metadata generation; suggest to delete {meta_file} to be recomputed"));
        }
        Ok(())
    }

    fn tar_builder(&self) -> Result<Builder<TarOutput>> {
        let output_file = match &self.output_file {
            Some(of) => of,
            None => return Err(anyhow!("No output file set")),
        };
        let offset = self.offset.unwrap_or_default();
        let tar_output = TarOutput::new(output_file,offset);
        let tar_builder = Builder::new(tar_output);
        Ok(tar_builder)
    }

    fn write_meta_file(&self) -> Result<()> {
        let meta_file = match &self.meta_file {
            Some(mf) => mf,
            None => return Err(anyhow!("No meta file set")),
        };
        let json = json!(self.entries);
        let mut output = File::create(meta_file)?;
        write!(output, "{}", json)?;
        Ok(())
    }
}


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path of file with list of files to tar
    #[arg(short, long)]
    input: String,

    /// Output tar file
    #[arg(short, long)]
    tar: String,

    /// Offset
    #[arg(short, long, default_value_t = 0)]
    offset: usize,
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    let mut fl = FileList::default();
    fl.set_output_file(&args.tar); // "foo.tar.test3"
    let files: Vec<String> = fs::read_to_string(args.input)
        .expect("Problem reading input file")
        .split("\n").map(|s|s.to_string())
        .collect();
    fl.set_files(&files).unwrap();
    fl.offset = Some(args.offset);
    fl.output().unwrap();
}
