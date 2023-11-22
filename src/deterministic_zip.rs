use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use zip::write::{FileOptions, ZipWriter};
use zip::{CompressionMethod, DateTime};

#[derive(Debug)]
pub enum Compression {
    None,
    Deflate,
}

impl Into<CompressionMethod> for Compression {
    fn into(self) -> CompressionMethod {
        match self {
            Compression::None => CompressionMethod::Stored,
            Compression::Deflate => CompressionMethod::Deflated,
        }
    }
}

#[derive(Debug)]
pub struct Opt {
    pub output: PathBuf,
    pub compression: Compression,
    pub paths: Vec<PathBuf>,
}

// #[paw::main]
// fn main(args: Opt) -> Result<(), std::io::Error> {
//     let paths: Vec<(PathBuf, PathBuf)> = args
//         .paths
//         .into_iter()
//         .flat_map(handle_path)
//         .map(|p| (p.clone(), p))
//         .collect();
//     let output_file = File::create(args.output)?;
//     create_zip_file(output_file, paths, args.compression.into(), args.quiet)?;
//     Ok(())
// }

// pub fn handle_path(path: PathBuf) -> Vec<PathBuf> {
//     if path.is_file() {
//         vec![path]
//     } else {
//         WalkDir::new(&path)
//             .follow_links(true)
//             .into_iter()
//             .filter_map(|e| e.ok())
//             .map(|e| e.into_path())
//             .collect()
//     }
// }

pub fn create_zip_file<W>(
    output_file: W,
    mut paths: Vec<(PathBuf, PathBuf)>,
    compression: CompressionMethod,
) -> Result<(), std::io::Error>
where
    W: Write + Seek,
{
    paths.sort();
    let options = FileOptions::default()
        .last_modified_time(DateTime::default())
        .compression_method(compression);
    let mut zip_writer = ZipWriter::new(output_file);

    let mut buffer = Vec::new();

    for (name, path) in paths {
        if path.is_file() {
            zip_writer.start_file(name.to_str().unwrap(), options)?;
            let mut f = File::open(path)?;
            f.read_to_end(&mut buffer)?;
            zip_writer.write_all(&*buffer)?;
            buffer.clear();
        } else {
            panic!("{} is not a file",path.display());
        }
    }

    zip_writer.finish()?;
    Ok(())
}
