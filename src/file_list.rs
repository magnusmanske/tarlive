use log::info;
use serde::{Serialize, Deserialize};
use zip::{ZipWriter, write::FileOptions, DateTime};
use std::{fs::{self, File}, io::{Write, Read}, env, sync::{Arc, Mutex}};
use anyhow::{anyhow, Result};
use serde_json::json;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use tar::Builder;

use crate::{file_entry::FileEntry, output_writer::OutputWriter};

const OVERHEAD_ESTIMATE_BYTES_PER_FILE: usize = 1024;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    #[default]
    Tar,
    Zip,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FileList {
    output_file: Option<String>,
    meta_file: Option<String>,
    entries: Vec<FileEntry>,
    offset: Option<usize>,
    end: Option<usize>,
    output_format: OutputFormat,
}

impl FileList {
    pub fn set_output_file(&mut self, filename: &str) {
        self.output_file = Some(filename.to_string());
    }

    pub fn set_output_format(&mut self, format: OutputFormat) {
        self.output_format = format;
    }

    pub fn set_offset(&mut self, offset: usize) {
        self.offset = match offset {
            0 => None,
            other => Some(other),
        };
    }

    pub fn set_end(&mut self, end: usize) {
        self.end = match end {
            0 => None,
            other => Some(other),
        };
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
        match self.output_format {
            OutputFormat::Tar => self.output_tar(),
            OutputFormat::Zip => self.output_zip(),
        }
    }

    pub fn output_tar(&mut self) -> Result<()>{
        let offset = self.offset.unwrap_or_default();
        let output_writer = OutputWriter::new(&self.output_file,offset,self.end.to_owned())?;
        let position = output_writer.position.clone();
        let mut output_writer = Builder::new(output_writer);

        output_writer.mode(tar::HeaderMode::Deterministic);
        for entry_id in 0..self.entries.len() {
            let entry = self.entries[entry_id].to_owned();
            let position_after = entry.tar_position_after;
            if self.is_earlier_than(&position, entry.len, offset) && position_after.is_some() {
                // Don't output anything, just advance byte counter
                *position.lock().unwrap() = position_after.unwrap(); // Safe
            } else {
                // Output file
                output_writer.append_path(&entry.path)?;
                self.entries[entry_id].tar_position_after = Some(*position.lock().unwrap());
                self.write_meta_file()?;
            }
        }
        let _dummy = output_writer.into_inner()?;
        Ok(())
    }

    pub fn output_zip(&mut self) -> Result<()>{
        if true {
            panic!("ZIP does not currently work");
        }
        // Create ZIP writer
        let offset = self.offset.unwrap_or_default();
        let output_writer = OutputWriter::new(&self.output_file,offset,self.end.to_owned())?;
        let position = output_writer.position.clone();
        let mut zip_writer = ZipWriter::new(output_writer);

        // ZIP options to make it deterministic
        let options = FileOptions::default()
            .last_modified_time(DateTime::default())
            .compression_method(zip::CompressionMethod::Stored);

        // Iterate over files
        for entry_id in 0..self.entries.len() {
            let entry = self.entries[entry_id].to_owned();
            let position_after = entry.tar_position_after;
            if self.is_earlier_than(&position, entry.len, offset) && position_after.is_some() {
                info!("Skipping {}", &entry.path);
                // Don't output anything, just advance byte counter
                *position.lock().unwrap() = position_after.unwrap(); // Safe
            } else {
                // Output file
                info!("Starting {}", &entry.path);
                zip_writer.start_file(&entry.path, options)?;

                let mut buffer = Vec::new();
                if true {
                    info!("Reading {}", &entry.path);
                    let mut f = File::open(&entry.path)?;
                    f.read_to_end(&mut buffer)?;
                    info!("Closing {}", &entry.path);
                }
                zip_writer.write_all(&*buffer)?;
                info!("ZIPped {}", &entry.path);

                self.entries[entry_id].tar_position_after = Some(*position.lock().unwrap());
                self.write_meta_file()?;
            }
        }

        // Finish ZIP file
        zip_writer.finish()?;
        Ok(())
    }

    pub fn is_earlier_than(&self, position: &Arc<Mutex<usize>>, file_size: usize, offset: usize) -> bool {
        *position.lock().unwrap() + OVERHEAD_ESTIMATE_BYTES_PER_FILE+file_size < offset
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
                let filename = format!("{}{base_64}.tarlive",tmp_dir.display());
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
