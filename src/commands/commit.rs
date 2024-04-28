use std::string::String;
use crate::data::data_structs::{Commit, CommittedFile};
use crate::utils::checks::{find_bucket, is_valid_bucket};
use crate::utils::config::{BucketConfig, RepositoryConfig};
use crate::utils::errors::BucketError;
use blake3::{Hash, Hasher};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};
use std::{env, io};
use log::{debug, error};
use rusqlite::{Connection, params};
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};
use zstd::Encoder;
use zstd::stream::copy_encode;
use crate::utils::checks;

pub(crate) fn execute() -> Result<(), BucketError> {
    // read repo config file
    #[allow(unused_variables)]
        let repo_config = RepositoryConfig::from_file(env::current_dir().unwrap())?;

    // find the top level of the bucket directory
    let current_path = env::current_dir()?;
    let bucket_path: PathBuf = match find_bucket(current_path.as_path()) {
        Some(mut path) => {
            path.pop();
            path
        }
        None => {
            return Err(BucketError::NotAValidBucket);
        }
    };

    // check if it is a valid bucket
    if !is_valid_bucket(bucket_path.as_path()) {
        return Err(BucketError::NotAValidBucket);
    }

    let bucket_config = BucketConfig::read_bucket_info(&bucket_path.join(".b"))?;

    // create a list of each file in the bucket directory, recursively
    // blake3 hash each file and add to metadata table
    let current_commit = generate_commit_metadata(bucket_path.as_path())?;
    if current_commit.files.is_empty() {
        println!("No files found in bucket. Commit cancelled.");
        return Ok(());
    }

    // if there are no difference with previous commit cancel commit
    match load_previous_commit(bucket_path.as_path(), &bucket_config) {
        Ok(None) => {
            process_files(bucket_config.id, bucket_path, &current_commit.files)?;
        }
        Ok(Some(previous_commit)) => {
            if let Some(changes) = current_commit.compare_commit(&previous_commit) {
                process_files(bucket_config.id, bucket_path, &changes)?;
            } else {
                println!("No changes detected. Commit cancelled.");
                return Ok(());
            }
        }
        Err(_) => {
            // Properly handle the error, perhaps by returning it
            error!("Failed to load previous commit");
        }
    }

    Ok(())
}

fn process_files(bucket_id: Uuid, bucket_path: PathBuf, files: &[CommittedFile]) -> Result<(), BucketError> {
    let db_location = checks::db_location(bucket_path.as_path());
    let conn = rusqlite::Connection::open(db_location)?;

    debug!("bucket id: {}", bucket_id.to_string().to_uppercase());
    let commit_id = insert_commit(&conn, bucket_id)?;

    let storage_path = bucket_path.join(".b").join("storage");
    for file in files {
        debug!("Processing file: {} {}", file.name, file.hash);
        let output = storage_path.join(&file.hash.to_string());
        insert_file(&conn, &commit_id, &file.name, &file.hash.to_string())?;
        // Replace unwrap with proper error handling
        compress_and_store_file(&file.name, output.as_path(), 0)?;
    }
    Ok(())
}

fn insert_file(conn: &Connection, commit_id: &str, file_path: &str, hash: &str) -> Result<(), BucketError> {
    conn.execute(
        "INSERT INTO files (commit_id, file_path, hash) VALUES (?1, ?2, ?3)",
        [commit_id, file_path, hash],
    )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error inserting into database: {}, commit id: {}, file path: {}, hash: {}", e, commit_id, file_path, hash),
            )
        })?;
    Ok(())
}

fn insert_commit(conn: &Connection, bucket_id: Uuid) -> Result<String, BucketError> {
    // Perform the insert operation without specifying an ID, which will trigger the auto-generation.
    conn.execute(
        "INSERT INTO commits (bucket_id) VALUES (?1)",
        [bucket_id.to_string().to_uppercase()],
    )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error inserting into database: {}, bucket id: {}", e, bucket_id.to_string().to_uppercase()),
            )
        })?;

    // Retrieve the last insert rowid, which is a feature of SQLite to get the rowid of the last inserted row.
    let last_row_id = conn.last_insert_rowid();

    // Now query back the `id` using the `rowid`
    let mut stmt = conn.prepare("SELECT id FROM commits WHERE rowid = ?1")?;
    let mut rows = stmt.query(params![last_row_id])?;

    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Err(BucketError::from(rusqlite::Error::QueryReturnedNoRows))
    }
}


fn load_previous_commit(bucket_path: &Path, bucket_config: &BucketConfig) -> Result<Option<Commit>, BucketError> {
    let db_location = checks::db_location(bucket_path);
    let conn = rusqlite::Connection::open(db_location)?;

    // todo: query all commits from a specific bucket
    let mut stmt = conn.prepare("SELECT f.id, f.file_path, f.hash
                                               FROM files f
                                               JOIN commits c ON f.commit_id = c.id
                                WHERE c.created_at = (SELECT MAX(created_at) FROM commits)")?;

    let mut rows = stmt.query([])?;

    let mut files = Vec::new();
    while let Some(row) = rows.next()? {
        let uuid_string: String = row.get(0)?;
        let hex_string: String = row.get(2)?;

        files.push(CommittedFile {
            id: uuid::Uuid::parse_str(&uuid_string).unwrap(),
            name: row.get(1)?,
            hash: Hash::from_hex(&hex_string).unwrap(),
            changed: false,
            new: false,
        });
    }

    Ok(Some(Commit {
        bucket: bucket_config.name.clone(),
        files,
        timestamp: "".to_string(),
    }))
}

fn compress_and_store_file(input_path: &str, output_path: &Path, compression_level: i32) -> io::Result<()> {
    let input_file = File::open(input_path)?;
    let output_file = File::create(output_path)?;
    let reader = BufReader::new(input_file);
    let writer = BufWriter::new(output_file);

    // Compress the file data and write it to the output file
    let mut encoder = Encoder::new(writer, compression_level)?;
    copy_encode(reader, &mut encoder, compression_level)?;
    encoder.finish()?;

    Ok(())
}

fn generate_commit_metadata(dir_path: &Path) -> io::Result<Commit> {
    let mut files = Vec::new();

    for entry in find_files_excluding_top_level_b(dir_path) {
        let path = entry.as_path();

        if path.is_file() {
            match hash_file(path) {
                Ok(hash) => {
                    //println!("BLAKE3 hash: {}", hash);
                    files.push(CommittedFile {
                        id: Default::default(),
                        name: path.to_string_lossy().into_owned(),
                        hash: hash,
                        new: false,
                        changed: false,
                    });
                }
                Err(e) => {
                    eprintln!("Failed to hash file: {}", e);
                    return Err(e);
                }
            }
        } else {
            debug!("Skipping non-file: {:?}", entry.as_path());
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

#[cfg(test)]
mod tests {
    use std::io::Write;
    use super::*;
    use chrono::DateTime;
    use tempfile::tempdir;

    #[test]
    fn test_generate_commit_metadata() -> io::Result<()> {
        let temp_dir = tempdir()?;
        std::env::set_current_dir(&temp_dir)?;

        let file_path = temp_dir.path().join("test_file.txt");
        let mut commited_file = File::create(&file_path)?;
        commited_file.write_all(b"Some content")?;

        let commit = generate_commit_metadata(temp_dir.path())?;

        // Asserts
        assert_eq!(commit.bucket, "");
        assert!(!commit.files.is_empty(), "No files found in commit");
        assert!(DateTime::parse_from_rfc3339(&commit.timestamp).is_ok(), "Invalid timestamp");

        for file in commit.files {
            assert!(file.name.contains("test_file.txt"), "File name mismatch");
            assert_eq!(file.hash.to_string(), "f4315de648c8440fb2539fe9a8417e901ab270a37c6e2267e0c5fffe7d4d4419", "Incorrect file hash");
        }

        Ok(())
    }
}