use std::{env, fs};
use std::fs::File;
use std::path::PathBuf;
use uuid::Uuid;
use crate::utils::checks::{find_directory_in_parents, is_valid_bucket};

pub(crate) fn execute() -> Result<(), std::io::Error> {

    // read repo config file
    #[allow(unused_variables)]
    let config = crate::utils::config::Config::from_file(env::current_dir().unwrap());

    // find the top level of the bucket directory
    let current_path = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            println!("Error getting current directory: {}", e);
            return Err(e);
        }
    };

    let path = find_directory_in_parents(current_path.as_path(), ".b");
    let mut bucket_path : PathBuf;


    match path {
        Some(path) => bucket_path = path,
        None => return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Not in a bucket directory",
        )),
    }

    // check if it is a valid bucket
    if !is_valid_bucket(bucket_path.as_path()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Not a valid bucket",
        ));
    }

    // create a temporary directory
    bucket_path = bucket_path.join("tmp");
    fs::create_dir_all(&bucket_path)?;

    // create storage directory with unique id in a temporary directory
    let unique_id = Uuid::new_v4().to_string();
    bucket_path = bucket_path.join(unique_id);
    fs::create_dir_all(&bucket_path)?;

    // create metadata file with timestamp in temporary directory
    let metadata_file_path = bucket_path.join("meta");

    #[allow(unused_variables)]
    let metadata_file = File::create(&metadata_file_path)?;

    // md5 hash each file and add to metadata file

    // if there are no difference with previous commit cancel commit

    // copy and compress files to storage directory
    // add filenames and original file sizes to metadata file

    // revert if error

    // move bucket directory out of temporary directory

    Ok(())
}