// use log::info;
use anyhow::Result;
use std::{fs::File, io::{self, Write}};

const OVERHEAD_ESTIMATE_BYTES_PER_FILE: usize = 1024;

pub struct TarOutput {
    output_file: Box<dyn Write>,
    offset: usize,
    pub position: usize,
    pub end: Option<usize>,
}

impl TarOutput {
    pub fn new(path: &Option<String>, offset: usize, end: Option<usize>) -> Result<Self> {
        Ok(Self {
            output_file :match path.to_owned().unwrap_or_default().as_str() {
                ""|"-" => Box::new(io::stdout()),
                path => Box::new(File::create(path)?),
            },
            offset,
            position: 0,
            end,
        })        
    }

    pub fn is_earlier_than(&self, file_size: usize) -> bool {
        self.position+OVERHEAD_ESTIMATE_BYTES_PER_FILE+file_size < self.offset
    }

    fn actual_write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let position = self.position;
        // info!("Writing from {} buflen {}",position,buf.len());
        if let Some(end) = self.end {
            if end>position && end<position+buf.len() {
                let len = end-position-1;
                // info!("pos: {}, end: {end}, len: {len}, buflen: {}",self.position, buf.len());
                self.output_file.write(&buf[..len])?;
                self.output_file.flush()?;
                self.position += len;
                let err = io::Error::new(io::ErrorKind::Other, "end position reached");
                return Err(err);
            }
        }
        let bytes_written = self.output_file.write(buf)?;
        self.position += bytes_written;
        Ok(bytes_written)
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
            let buf_start = self.offset - self.position - 1;
            self.position += buf_start; // Pretend we wrote this too
            // info!("Starting partial from {} with buf_start {buf_start} len {len} offset {}",self.position,self.offset);
            let bytes_written = self.actual_write(&buf[buf_start..len])?;
            // info!("{buf_start}+{bytes_written}={len}? {}",len-buf_start-bytes_written);
            return Ok(buf_start+bytes_written);
        }
        self.actual_write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output_file.flush()
    }
}
