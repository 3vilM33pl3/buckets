use crate::utils::checks::{find_directory_in_parents, is_valid_bucket};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{env, fs, io};
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};
use crate::commands::create::BucketConfig;
use crate::data::data_structs::{Commit, CommittedFile};
use crate::utils::checks;
use crate::utils::errors::BucketError;
use blake3::{Hasher, Hash};

pub(crate) fn execute() -> Result<(), BucketError> {
    // read repo config file
    #[allow(unused_variables)]
    let config = crate::utils::config::Config::from_file(env::current_dir().unwrap());

    // find the top level of the bucket directory
    let current_path = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            println!("Error getting current directory: {}", e);
            return Err(BucketError::IoError(e));
        }
    };

    let path = find_directory_in_parents(current_path.as_path(), ".b");
    let bucket_path: PathBuf;

    match path {
        Some(mut path) =>  {
            path.pop();
            bucket_path = path;
        },
        None => {
            return Err(BucketError::NotAValidBucket);
        }
    }


    // check if it is a valid bucket
    if !is_valid_bucket(bucket_path.as_path()) {
        return Err(BucketError::NotAValidBucket);
    }

    // create a temporary directory
    let tmp_bucket_path = bucket_path.join(".b").join("tmp");
    fs::create_dir_all(&tmp_bucket_path)?;

    // create storage directory with unique id in a temporary directory
    let unique_id = Uuid::new_v4().to_string();
    let unique_bucket_path = tmp_bucket_path.join(unique_id);
    fs::create_dir_all(&unique_bucket_path)?;

    // create a list of each file in the bucket directory, recursively
    // md5 hash each file and add to metadata table
    let current_commit = generate_commit_data(bucket_path.as_path())?;
    current_commit.files.iter().for_each(|file| {
        println!("{} {}", file.name, file.hash);
    });

    // if there are no difference with previous commit cancel commit
    match load_previous_commit(BucketConfig::read_bucket_config(&bucket_path.join(".b"))?) {
        Ok(None) => {}
        Ok(Some(previous_commit)) => {
            let changes = current_commit.compare_commit(&previous_commit);
            match changes {
                Some(changes) => changes.iter().for_each(|file| {
                    println!("{} {}", file.name, file.hash);
                }),
                None => {
                    println!("No changes");
                    return Ok(());
                }
            }
        }
        Err(_) => {}
    };

    // copy and compress files to storage directory
    // add filenames and original file sizes to metadata file



    // rollback if error

    // create metadata file with timestamp in temporary directory
    let metadata_file_path = bucket_path.join("meta");
    #[allow(unused_variables)]
    let metadata_file = File::create(&metadata_file_path)?;

    // move bucket directory out of temporary directory


    Ok(())
}

fn load_previous_commit(bucket_config: BucketConfig) -> Result<Option<Commit>, BucketError> {

    let db_location = checks::db_location(bucket_config.path.as_path());
    let conn = rusqlite::Connection::open(db_location)?;

    // todo: query all commits from a specific bucket
    let mut stmt = conn.prepare("SELECT * FROM commits ORDER BY timestamp DESC LIMIT 1")?;

    #[allow(unused_variables)]
    let rows = stmt.query([])?;

    return Ok(None);
}

fn generate_commit_data(dir_path: &Path) -> io::Result<Commit> {
    let mut files = Vec::new();

    for entry in find_files_excluding_top_level_b(dir_path) {

        let path = entry.as_path();

        if path.is_file() {
            match hash_file(path) {
                Ok(hash) => {
                    println!("BLAKE3 hash: {}", hash);
                    files.push(CommittedFile {
                        name: path.to_string_lossy().into_owned(),
                        hash: hash,
                    });
                },
                Err(e) => {
                    eprintln!("Failed to hash file: {}", e);
                    return Err(e);
                }
            }


        }
    }

    Ok(Commit {
        bucket: "".to_string(),
        files,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

fn hash_file<P: AsRef<Path>>(path: P) -> io::Result<Hash> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buffer = [0; 1024]; // Buffer for reading chunks

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break; // End of file
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hasher.finalize())
}

fn find_files_excluding_top_level_b(dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| is_valid_file(entry, dir))
        .filter_map(|entry| make_relative_path(entry.path(), dir))
        .collect()
}

fn is_valid_file(entry: &DirEntry, root_dir: &Path) -> bool {
    let is_top_level_b = entry.depth() == 1 && entry.file_name() == ".b";
    let is_inside_top_level_b = entry.path().starts_with(&root_dir.join(".b"));

    entry.file_type().is_file() && !is_top_level_b && !is_inside_top_level_b
}

fn make_relative_path(path: &Path, base: &Path) -> Option<PathBuf> {
    path.strip_prefix(base).ok().map(PathBuf::from)
}