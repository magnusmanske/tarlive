use log::info;
use serde::{Serialize, Deserialize};
use std::{fs::{self, File}, io::Write, env};
use anyhow::{anyhow, Result};
use serde_json::json;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use tar::Builder;

use crate::{file_entry::FileEntry, tar_output::TarOutput};

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

    pub fn set_offset(&mut self, offset: usize) {
        self.offset = Some(offset);
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
        let offset = self.offset.unwrap_or_default();
        let tar_output = TarOutput::new(&self.output_file,offset);
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
