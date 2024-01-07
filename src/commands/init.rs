use crate::utils::checks;
use crate::utils::config::create_default_config;
use std::{env, fs};

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
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Directory is not empty",
        ));
    }

    match checks::find_directory_in_parents(current_path.as_path(), ".buckets") {
        Some(found_path) => println!(
            "Can not initialise, already in a buckets repository: {}",
            found_path.display()
        ),
        _ => {}
    }

    let init_dir_path = current_path.join(".buckets");
    fs::create_dir_all(&init_dir_path)?;

    // Create the buckets.conf file
    create_default_config(init_dir_path.as_path());

    // let now = Utc::now();
    // file.write_fmt(format_args!("{}", now.to_rfc3339()))?;

    Ok(())
}
