use log::info;
use anyhow::Result;
use std::{fs::File, io::{self, Write, Seek, SeekFrom}, sync::{Arc, Mutex}};

pub struct OutputWriter {
    output_file: Box<dyn Write>,
    offset: usize,
    max_written: usize,
    pub position: Arc<Mutex<usize>>,
    pub end: Option<usize>,
}

impl OutputWriter {
    pub fn new(path: &Option<String>, offset: usize, end: Option<usize>) -> Result<Self> {
        Ok(Self {
            output_file :match path.to_owned().unwrap_or_default().as_str() {
                ""|"-" => Box::new(io::stdout()),
                path => Box::new(File::create(path)?),
            },
            offset,
            position: Arc::new(Mutex::new(0)),
            max_written: 0,
            end,
        })        
    }

    fn actual_write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // info!("Writing from {} buflen {}",position,buf.len());
        if let Some(end) = self.end {
            if end > *self.position.lock().unwrap() && end < *self.position.lock().unwrap() + buf.len() {
                let len = end - *self.position.lock().unwrap() - 1;
                // info!("pos: {}, end: {end}, len: {len}, buflen: {}",self.position, buf.len());
                self.output_file.write(&buf[..len])?;
                self.output_file.flush()?;
                *self.position.lock().unwrap() += len;
                let err = io::Error::new(io::ErrorKind::Other, "end position reached");
                return Err(err);
            }
        }
        let bytes_written = self.output_file.write(buf)?;
        *self.position.lock().unwrap() += bytes_written;
        if self.max_written < *self.position.lock().unwrap() {
            self.max_written = *self.position.lock().unwrap();
        }
        Ok(bytes_written)
    }
}

impl Write for OutputWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();
        if *self.position.lock().unwrap()+len < self.offset {
            *self.position.lock().unwrap() += len;
            if self.max_written < *self.position.lock().unwrap() {
                self.max_written = *self.position.lock().unwrap();
            }
            return Ok(len);
        }
        if *self.position.lock().unwrap() < self.offset {
            let buf_start = self.offset - *self.position.lock().unwrap() - 1;
            *self.position.lock().unwrap() += buf_start; // Pretend we wrote this too
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


impl Seek for OutputWriter {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        info!("Seeking {pos:?}");
        // TODO Err on invalid positions eg <0
        match pos {
            SeekFrom::Start(pos) => *self.position.lock().unwrap() = pos as usize,
            SeekFrom::End(pos) => *self.position.lock().unwrap() = ((self.max_written as i64) + pos) as usize,
            SeekFrom::Current(0) => {} // Nothing to do
            SeekFrom::Current(pos) => {
                let current_position = *self.position.lock().unwrap();
                *self.position.lock().unwrap() = ((current_position as i64) + pos) as usize;
            }
        }
        let pos = *self.position.lock().unwrap();
        // info!("Now at {pos}");
        if pos >= self.offset {
            // self.output_file.seek(SeekFrom::Start(pos-self.offset));
        }
        Ok(pos as u64)
    }
}