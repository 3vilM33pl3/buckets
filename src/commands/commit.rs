use std::string::String;
use crate::data::commit::{Commit, CommittedFile};
use crate::utils::config::{RepositoryConfig};
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
use crate::data::bucket::Bucket;
use crate::utils::checks;

// Execute the `commit` command
pub(crate) fn execute(message: &String) -> Result<(), BucketError> {
    // read repo config file
    #[allow(unused_variables)]
        let repo_config = RepositoryConfig::from_file(env::current_dir().unwrap())?;

    let bucket = match Bucket::from_meta_data(env::current_dir()?) {
        Ok(bucket) => bucket,
        Err(e) => {
            eprintln!("Error reading bucket info: {}", e);
            return Err(e);
        }
    };

    // create a list of each file in the bucket directory, recursively
    // and create a blake3 hash for each file and add to current_commit
    let current_commit = generate_commit_metadata(&bucket)?;
    if current_commit.files.is_empty() {
        println!("No files found in bucket. Commit cancelled.");
        return Ok(());
    }

    // Load the previous commit, if it exists
    match load_last_commit(&bucket) {
        Ok(None) => {
            // There is no previous commit; Process all files in the current commit
            process_files(bucket.id, &bucket.relative_bucket_path, &current_commit.files, message)?;
        }
        Ok(Some(previous_commit)) => {
            // Compare the current commit with the previous commit
            if let Some(changes) = current_commit.compare(&previous_commit) {
                // Process the files that have changed
                process_files(bucket.id, &bucket.relative_bucket_path, &changes, message)?;
            } else {
                // if there are no difference with previous commit cancel commit
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

/// Processes a list of files by inserting commit and file data into a database and optionally handling file storage.
///
/// This function coordinates several operations essential for version control management:
/// - It opens a database connection using a path derived from `bucket_path`.
/// - It inserts a new commit record into the database.
/// - It processes each file in the provided list by inserting file metadata into the database and handling
///   physical file storage and compression as necessary.
///
/// # Arguments
/// * `bucket_id` - The UUID of the bucket under which these files and commit are categorized.
/// * `bucket_path` - The file system path to the bucket, used to determine the database location and storage paths.
/// * `files` - A slice of `CommittedFile` structs representing the files to be processed.
///
/// # Returns
/// Returns a `Result<(), BucketError>` indicating the success or failure of the processing operations:
/// - `Ok(())`: All files were processed successfully, including database insertions and file storage.
/// - `Err(BucketError)`: If any operation fails, including database errors, file IO errors, or data serialization issues.
///
/// # Errors
/// The function can encounter various errors:
/// - Database connection issues or failures during SQL command execution.
/// - Errors from the `insert_commit` or `insert_file` functions if they encounter issues.
/// - File system errors when creating directories or handling files, such as permissions errors or disk space limitations.
///
/// # Example Usage
/// ```
/// use uuid::Uuid;
/// use std::path::PathBuf;
///
/// let bucket_id = Uuid::new_v4();
/// let bucket_path = PathBuf::from("/path/to/bucket");
/// let files = vec![
///     CommittedFile {
///         id: Uuid::new_v4(),
///         name: "example.txt".to_string(),
///         hash: "dummy_hash".to_string(),
///         new: false,
///         changed: false,
///     },
/// ];
///
/// match process_files(bucket_id, bucket_path, &files) {
///     Ok(_) => println!("Files processed successfully."),
///     Err(e) => eprintln!("Failed to process files: {}", e),
/// }
/// ```
// Process the files in the commit
fn process_files(bucket_id: Uuid, bucket_path: &PathBuf, files: &[CommittedFile], message: &String) -> Result<(), BucketError> {
    // Open the database connection
    let db_location = checks::db_location(bucket_path.as_path());
    let conn = rusqlite::Connection::open(db_location)?;

    // Insert the commit into the database
    debug!("bucket id: {}", bucket_id.to_string().to_uppercase());
    let commit_id = insert_commit(&conn, bucket_id, message)?;

    // Create the storage directory
    let storage_path = bucket_path.join(".b").join("storage");

    // Process each file in the commit
    for file in files {
        debug!("Processing file: {} {}", file.name, file.hash);
        let output = storage_path.join(&file.hash.to_string());

        // Insert the file into the database
        insert_file(&conn, &commit_id, &file.name, &file.hash.to_string())?;

        // TODO: Replace unwrap with proper error handling
        // Compress and store the file
        compress_and_store_file(&file.name, output.as_path(), 0)?;
    }
    Ok(())
}

/// Inserts file metadata into the `files` table of the database.
///
/// This function adds a new record to the `files` table with the specified `commit_id`, `file_path`, and `hash`.
/// It is designed to store metadata about files associated with a specific commit in a version control system.
///
/// # Arguments
/// * `conn` - A reference to an open SQLite `Connection`. This connection must be to a database that has the
///   `files` table configured correctly.
/// * `commit_id` - A string slice that holds the UUID of the commit this file is associated with. This ID must
///   correspond to a valid commit ID already present in the `commits` table.
/// * `file_path` - A string slice representing the path of the file relative to the repository root.
/// * `hash` - A string slice representing the hash of the file content, used to verify file integrity.
///
/// # Returns
/// Returns a `Result<(), BucketError>`:
/// - `Ok(())`: Indicates that the file metadata was successfully inserted into the database.
/// - `Err(BucketError)`: If an error occurs during the SQL execution, a `BucketError` is returned detailing
///   the nature of the error.
///
/// # Errors
/// Errors can arise from:
/// - SQL execution failure: If the INSERT statement fails (due to reasons such as SQL syntax errors, database locks,
///   or foreign key constraints).
/// - Failure in preparing or executing the SQL command.
///
/// # Example Usage
/// ```
/// use rusqlite::Connection;
///
/// let conn = Connection::open("my_database.db").unwrap();
/// let commit_id = "1b4e28ba-2fa1-11d2-883f-0016d3cca427";
/// let file_path = "src/main.rs";
/// let hash = "c3ab8ff13720e8ad9047dd39466b3c894bdfa1a3";
/// match insert_file(&conn, commit_id, file_path, hash) {
///     Ok(_) => println!("File metadata inserted successfully."),
///     Err(e) => eprintln!("Failed to insert file metadata: {}", e),
/// }
/// ```
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

/// Inserts a new commit record into the database with the specified `bucket_id` and retrieves the auto-generated commit ID.
///
/// This function performs an SQL INSERT operation to create a new commit record associated with a given bucket.
/// The commit ID is auto-generated by the database. After inserting, the function fetches the generated ID
/// by querying the database using the last inserted row's rowid, which is unique to each session.
///
/// # Arguments
/// * `conn` - A reference to an open SQLite `Connection`. This connection must be to a database that has the
///   `commits` table configured correctly.
/// * `bucket_id` - The `Uuid` of the bucket to which this commit belongs. This UUID should already exist in the
///   database under the `buckets` table or the relevant foreign key table.
///
/// # Returns
/// Returns a `Result<String, BucketError>`:
/// - `Ok(String)`: The function returns the UUID string of the newly inserted commit if the operation is successful.
/// - `Err(BucketError)`: If any errors occur during the SQL execution or while fetching the commit ID, a `BucketError`
///   is returned detailing the nature of the error.
///
/// # Errors
/// Errors can arise from:
/// - SQL execution failure: If the INSERT statement fails (due to reasons like SQL syntax errors, database locks,
///   or foreign key constraints).
/// - Failure in fetching the auto-generated ID: If there are issues retrieving the last insert rowid or querying it
///   to get the commit ID.
///
/// # Example Usage
/// ```
/// use rusqlite::Connection;
/// use uuid::Uuid;
///
/// let conn = Connection::open("my_database.db").unwrap();
/// let bucket_id = Uuid::parse_str("1b4e28ba-2fa1-11d2-883f-0016d3cca427").unwrap();
/// match insert_commit(&conn, bucket_id) {
///     Ok(commit_id) => println!("Inserted commit with ID: {}", commit_id),
///     Err(e) => eprintln!("Failed to insert commit: {}", e),
/// }
/// ```
fn insert_commit(conn: &Connection, bucket_id: Uuid, message: &String) -> Result<String, BucketError> {
    // Perform the insert operation without specifying an ID, which will trigger the auto-generation.
    conn.execute(
        "INSERT INTO commits (bucket_id, message) VALUES (?1, ?2)",
        [bucket_id.to_string().to_uppercase(), message.parse().unwrap()],
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

/// Loads the most recent commit and its associated files from the database for a specified bucket.
///
/// This function accesses the SQLite database located at the path determined by the `bucket_path`.
/// It retrieves all files associated with the most recent commit in the specified bucket. The files' metadata
/// and the commit details are returned as a `Commit` struct wrapped in an `Option`. If there are no commits
/// yet, it returns `None`.
///
/// # Arguments
/// * `bucket_path` - A reference to a `Path` that specifies the directory containing the bucket.
/// * `bucket_config` - A reference to a `BucketConfig` struct containing configuration details of the bucket,
///   such as its name.
///
/// # Returns
/// Returns a `Result` wrapping an `Option<Commit>`. On success, it contains:
/// - `Some(Commit)`: A `Commit` struct containing details of the latest commit and its files.
/// - `None`: If no commits are found in the database.
///
/// # Errors
/// Returns a `BucketError` if any errors occur during database access, query execution, or while reading
/// the data from the database. This can include:
/// - Database connection failures.
/// - SQL preparation or execution errors.
/// - Data parsing errors, such as failing to parse UUIDs or hexadecimal strings.
///
/// # Example Usage
/// ```
/// use std::path::Path;
/// let bucket_path = Path::new("/path/to/bucket");
/// let bucket_config = BucketConfig { name: "example_bucket".to_string() };
/// match load_previous_commit(bucket_path, &bucket_config) {
///     Ok(Some(commit)) => println!("Loaded commit with {} files.", commit.files.len()),
///     Ok(None) => println!("No commits found."),
///     Err(e) => eprintln!("Error loading commits: {}", e),
/// }
/// ```
fn load_last_commit(bucket: &Bucket) -> Result<Option<Commit>, BucketError> {
    let db_location = checks::db_location(bucket.relative_bucket_path.join(".b").as_path());
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
        bucket: bucket.name.clone(),
        files,
        timestamp: "".to_string(),
        previous: None,
        next: None,
    }))
}

/// Compresses a file from a specified input path and stores the compressed data at an output path.
///
/// This function reads a file from `input_path`, compresses it using the specified `compression_level`,
/// and writes the compressed data to a file located at `output_path`. The compression is handled using
/// a zstd encoder, which is capable of high compression ratios and fast processing speeds suitable for
/// a wide range of data types.
///
/// # Arguments
/// * `input_path` - A string slice that holds the path to the input file to be compressed.
/// * `output_path` - A reference to a `Path` that specifies where the compressed file should be stored.
/// * `compression_level` - An integer specifying the compression level to be used by the encoder.
///    A higher value results in better compression at the cost of speed. Typical values range from
///    1 (fastest, less compression) to 22 (slowest, best compression).
///
/// # Returns
/// This function returns an `io::Result<()>`. On successful execution, it returns `Ok(())`. If it encounters
/// any I/O errors during file operations or compression, it returns an `Err` variant containing an `io::Error`.
///
/// # Errors
/// Errors can arise from:
/// - Problems opening the input file, such as file not found, lacking read permissions, or the file being locked.
/// - Issues creating the output file, like inadequate write permissions or disk space issues.
/// - Failures during the read, write, or compression processes, such as corrupted data or an interrupted process.
///
/// # Example Usage
/// ```
/// use std::path::Path;
/// let result = compress_and_store_file("path/to/input/file.txt", Path::new("path/to/output/file.zst"), 3);
/// match result {
///     Ok(_) => println!("File compressed and stored successfully."),
///     Err(e) => eprintln!("Failed to compress and store file: {}", e),
/// }
/// ```
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

/// Generates metadata for a commit based on files found in a directory, excluding top-level `.b` directory files.
///
/// This function traverses a specified directory and constructs metadata for a new commit. It includes
/// information about each file found (excluding those in the `.b` directory), such as file hashes and names.
/// It collects these into a `Commit` struct, which also includes a timestamp marking the creation of the commit.
///
/// # Arguments
/// * `dir_path` - A reference to a `Path` that specifies the directory to scan for files.
///
/// # Returns
/// Returns a `Result` wrapping a `Commit` struct on success, containing:
/// - `bucket`: a placeholder for bucket identification, currently initialized to an empty string.
/// - `files`: a vector of `CommittedFile` structs representing each file's metadata in the commit.
/// - `timestamp`: a UTC timestamp in RFC3339 format indicating when the commit was generated.
///
/// If the function encounters an error while hashing any file, it will return an `io::Error` immediately,
/// halting further processing and indicating the nature of the failure.
///
/// # Errors
/// Errors can arise from:
/// - File access issues, such as permissions errors or files being locked.
/// - IO issues when reading files during hashing.
///
/// # Examples
/// ```
/// use std::path::Path;
/// let commit_metadata = generate_commit_metadata(Path::new("./some/directory"));
/// match commit_metadata {
///     Ok(commit) => println!("Generated commit metadata successfully!"),
///     Err(e) => eprintln!("Error generating commit metadata: {}", e),
/// }
/// ```
fn generate_commit_metadata(bucket: &Bucket) -> io::Result<Commit> {
    let mut files = Vec::new();

    for entry in find_files_excluding_top_level_b(bucket.relative_bucket_path.as_path()) {
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
        previous: None,
        next: None,
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

        let bucket = &Bucket::default(uuid::Uuid::new_v4(), &"test_bucket".to_string(), &temp_dir.path().to_path_buf());

        let commit = generate_commit_metadata(bucket)?;

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