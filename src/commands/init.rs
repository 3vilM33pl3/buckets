use std::{env, fs};
use std::fs::File;
use std::io::Write;
use std::path::{Path};
use crate::utils::checks;
use chrono::Utc;

pub fn execute() -> Result<(), std::io::Error> {
    println!("Initialising bucket repositiry");

    let current_path = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            println!("Error getting current directory: {}", e);
            return Err(e);
        }
    };

    if !checks::is_directory_empty(current_path.as_path()) {
        println!("Directory is not empty: {}", current_path.display());
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Directory is not empty"));
    }

    match checks::find_directory_in_parents(current_path.as_path(), ".buckets") {
        Some(found_path) => println!("Can not initialise, already a buckets repository: {}", found_path.display()),
        _ => {}
    }

    let init_dir_path = Path::new(".init");
    fs::create_dir_all(&init_dir_path)?;

    // Create the buckets.conf file
    let conf_file_path = init_dir_path.join("buckets.conf");
    let mut file = File::create(&conf_file_path)?;

    let now = Utc::now();
    file.write_fmt(format_args!("{}", now.to_rfc3339()))?;

    Ok(())
}