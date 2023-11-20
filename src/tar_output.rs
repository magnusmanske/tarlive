use std::{fs::File, io::{self, Write}};

const OVERHEAD_ESTIMATE_BYTES_PER_FILE: usize = 1024;

#[derive(Debug)]
pub struct TarOutput {
    output_file: File,
    offset: usize,
    pub position: usize,
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
