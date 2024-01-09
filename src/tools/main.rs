use std::{env, fs};

fn main() {
    // create directory
    let path = env::current_dir().unwrap().join("test_repo");
    if path.exists() && path.is_dir() {
        fs::remove_dir_all(&path).expect("Unable to remove directory");
    }
    fs::create_dir_all(&path).expect("Unable to create directory");
}