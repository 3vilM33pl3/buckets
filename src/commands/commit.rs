use crate::utils::checks::{find_directory_in_parents, is_valid_bucket};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{env, fs, io};
use uuid::Uuid;
use walkdir::WalkDir;
use crate::data::data_structs::{Commit, CommittedFile};
use crate::utils::checks;
use crate::utils::errors::BucketError;

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
    let bucket_path: PathBuf;

    match path {
        Some(path) => bucket_path = path,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Not in a bucket directory",
            ))
        }
    }

    // check if it is a valid bucket
    if !is_valid_bucket(bucket_path.as_path()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Not a valid bucket",
        ));
    }

    // create a temporary directory
    let tmp_bucket_path = bucket_path.join("tmp");
    fs::create_dir_all(&bucket_path)?;

    // create storage directory with unique id in a temporary directory
    let unique_id = Uuid::new_v4().to_string();
    #[allow(unused_variables)]
    let unique_bucket_path = tmp_bucket_path.join(unique_id);
    fs::create_dir_all(&bucket_path)?;

    // create a list of each file in the bucket directory, recursively
    // md5 hash each file and add to metadata file
    let current_commit = generate_meta_for_directory(bucket_path.as_path())?;
    current_commit.files.iter().for_each(|file| {
        println!("{} {}", file.name, file.md5);
    });

    // if there are no difference with previous commit cancel commit
    match load_previous_commit(bucket_path.as_path()) {
        Ok(None) => {}
        Ok(Some(previous_commit)) => {
            let changes = current_commit.compare_commit(&previous_commit);
            match changes {
                Some(changes) => changes.iter().for_each(|file| {
                    println!("{} {}", file.name, file.md5);
                }),
                None => {
                    println!("No changes");
                    return Ok(());
                }
            }
        }
        Err(_) => {}
    };



    // {
    //     None => {}
    //     Some(previous_commit) => {
    //         let changes = current_commit.compare_commit(&previous_commit);
    //         match changes {
    //             Some(changes) => changes.iter().for_each(|file| {
    //                 println!("{} {}", file.name, file.md5);
    //             }),
    //             None => {
    //                 println!("No changes");
    //                 return Ok(());
    //             }
    //         }
    //     }
    //     (_) => {}
    // };

    // copy and compress files to storage directory
    // add filenames and original file sizes to metadata file


    // revert if error

    // create metadata file with timestamp in temporary directory
    let metadata_file_path = bucket_path.join("meta");
    #[allow(unused_variables)]
    let metadata_file = File::create(&metadata_file_path)?;


    // move bucket directory out of temporary directory


    Ok(())
}

fn load_previous_commit(bucket_path: &Path) -> Result<Option<Commit>, BucketError> {

    let db_location = checks::db_location(bucket_path);
    let conn = rusqlite::Connection::open(db_location)?;

    let mut stmt = conn.prepare("SELECT * FROM commits ORDER BY timestamp DESC LIMIT 1")?;

    #[allow(unused_variables)]
    let rows = stmt.query([])?;

    match find_directory_in_parents(bucket_path, ".b") {
        None => {
            println!("Not in a bucket directory");
            return Ok(None);
        }
        Some(_) => {}
    }
    return Ok(None);
}

fn generate_meta_for_directory<P: AsRef<Path>>(dir_path: P) -> io::Result<Commit> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir_path) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let mut file = fs::File::open(path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;

            let md5 = md5::compute(&buffer);
            let md5_str = format!("{:x}", md5);

            files.push(CommittedFile {
                name: path.to_string_lossy().into_owned(),
                size: buffer.len() as u64,
                md5: md5_str,
            });
        }
    }

    Ok(Commit {
        bucket: "".to_string(),
        files,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
