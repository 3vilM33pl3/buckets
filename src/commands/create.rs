use crate::utils::checks;
use serde::Serialize;
use std::env;
use std::fs::{create_dir_all, File};
use std::io::Write;
use toml::to_string;
#[derive(Serialize)]
struct BucketConfig {
    pub name: String,
}

impl BucketConfig {
    fn default(name: &String) -> Self {
        BucketConfig {
            name: name.to_string(),
        }
    }
}
pub fn execute(bucket_name: &String) -> Result<(), std::io::Error> {
    println!("Creating bucket");

    let current_path = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            println!("Error getting current directory: {}", e);
            return Err(e);
        }
    };

    // check if in buckets repository
    if !checks::is_valid_bucket_repo(current_path.as_path()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Not in a bucket repository",
        ));
    }

    // check if bucket already exists
    let path = current_path.join(bucket_name).join(".b");
    if path.exists() && path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Bucket already exists",
        ));
    }

    // create bucket with given name
    create_dir_all(path.as_path())?;

    // add info to bucket hidden directory
    let config = BucketConfig::default(bucket_name);
    let toml_string = to_string(&config).unwrap();
    let mut file = File::create(path.join("info")).unwrap();
    file.write_all(toml_string.as_bytes()).unwrap();

    Ok(())
}
