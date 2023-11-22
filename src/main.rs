use clap::Parser;
use file_list::{FileList, OutputFormat};
use std::{fs::{self}, io};

pub mod file_entry;
pub mod output_writer;
pub mod file_list;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path of file with list of files to tar
    #[arg(short, long, default_value_t=String::new())]
    input: String,

    /// Output tar file
    #[arg(short, long, default_value_t=String::new())]
    tar: String,

    /// Offset
    #[arg(short, long, default_value_t = 0)]
    offset: usize,

    /// End
    #[arg(short, long, default_value_t = 0)]
    end: usize,

    /// Format
    #[arg(short, long, default_value_t = format!("tar"))]
    format: String,
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    let mut fl = FileList::default();
    fl.set_output_file(&args.tar);
    let files: Vec<String> = if args.input=="-" || args.input.is_empty() {
        io::stdin().lines().filter_map(|l|l.ok()).collect()
    } else {
        fs::read_to_string(args.input)
            .expect("Problem reading input file")
            .split("\n").map(|s|s.to_string())
            .collect()
    };
    match args.format.trim().to_lowercase().as_str() {
        ""|"tar" => fl.set_output_format(OutputFormat::Tar),
        // "zip" => fl.set_output_format(OutputFormat::Zip),
        _ => panic!("Unsupported output format \"{}\"",args.format),
    }
    fl.set_files(&files).unwrap();
    fl.set_offset(args.offset);
    fl.set_end(args.end);
    match fl.output() {
        Ok(_) => {},
        Err(e) => log::info!("{e}"),
    }
}
