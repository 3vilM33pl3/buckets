use clap::{Parser};
use rand::{distributions::Alphanumeric, Rng};
use std::fs::{self, File};
use std::io::Write;
use std::path::{PathBuf, Path};

/// Creates multiple files with random text content in a specified directory.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The directory where files will be created.
    #[clap(short, long)]
    directory: String,

    /// The prefix for each file name.
    #[clap(short, long)]
    prefix: String,

    /// The number of files to create.
    #[clap(short, long)]
    number: usize,

    /// The length of the random text in each file.
    #[clap(short, long, default_value_t = 100)]
    length: usize,
}

#[allow(dead_code)]
fn create_unique_file_path(base_path: &Path, prefix: &str, idx: usize) -> PathBuf {
    let mut counter = 0;
    loop {
        let file_name = format!("{}{}{}.txt", prefix, idx, if counter > 0 { format!("_{}", counter) } else { String::new() });
        let file_path = base_path.join(file_name);
        if !file_path.exists() {
            return file_path;
        }
        counter += 1;
    }
}

#[allow(dead_code)]
fn main() {
    let args = Args::parse();

    let path = PathBuf::from(&args.directory);
    fs::create_dir_all(&path).expect("Failed to create directory");

    for i in 0..args.number {
        let file_path = create_unique_file_path(&path, &args.prefix, i);
        let mut file = File::create(&file_path).expect("Failed to create file");
        let random_text: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(args.length)
            .map(char::from)
            .collect();

        writeln!(file, "File Number: {}\nRandom Content: {}", i, random_text).expect("Failed to write to file");
        println!("Created file: {}", file_path.display());
    }
}
