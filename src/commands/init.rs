use crate::utils::checks;
use crate::utils::config::create_default_config;
use std::{env, fs};

pub fn execute(repo_name: &String) -> Result<(), std::io::Error> {
    println!("Initialising bucket repository");

    let current_path = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            println!("Error getting current directory: {}", e);
            return Err(e);
        }
    };

    // check if already in a bucket repository
    match checks::find_directory_in_parents(current_path.as_path(), ".buckets") {
        Some(found_path) => println!(
            "Can not initialise, already in a buckets repository: {}",
            found_path.display()
        ),
        _ => {}
    }

    // Check if directory with the same name exists
    let path = current_path.join(repo_name);
    if path.exists() && path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Directory with same name already exists",
        ));
    }


    // Create the .buckets directory
    let init_dir_path = path.join(".buckets");
    fs::create_dir_all(&init_dir_path)?;

    // Create the buckets.conf file
    create_default_config(init_dir_path.as_path());

    // let now = Utc::now();
    // file.write_fmt(format_args!("{}", now.to_rfc3339()))?;

    Ok(())
}
