use std::{fs::File, io::{self, Write}};

const OVERHEAD_ESTIMATE_BYTES_PER_FILE: usize = 1024;

// #[derive(Debug)]
pub struct TarOutput {
    output_file: Box<dyn Write>,
    offset: usize,
    pub position: usize,
}

impl TarOutput {
    pub fn new(path: &Option<String>, offset: usize) -> Self {
        let output_file: Box<dyn Write> = match path {
            Some(path) => {
                match path.as_str() {
                    "" => Box::new(io::stdout()),
                    "-" => Box::new(io::stdout()),
                    path => Box::new(File::create(path).unwrap()),
                }
                    
            }
            None => Box::new(io::stdout()),
        };
        Self {
            output_file,
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
