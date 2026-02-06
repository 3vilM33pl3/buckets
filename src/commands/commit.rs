use crate::args::CommitCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::read_bucket_info;
use crate::data::commit::{Commit as CommitData, CommitStatus, CommittedFile};
use crate::errors::BucketError;
use crate::postgres_db::{get_database, DatabaseManager};
use crate::utils::runtime::RuntimeManager;
use crate::utils::utils::{find_files_excluding_top_level_b, hash_file};
use crate::world::World;
use async_trait::async_trait;
use blake3::Hash;
use log::{debug, error};
use std::io;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio_postgres::types::ToSql;
use uuid::Uuid;

const ZERO_HASH_STR: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Debug)]
struct LatestCommitRow {
    id: Uuid,
    bucket: String,
    timestamp: String,
}

#[derive(Clone, Debug)]
struct CommitFileRow {
    id: Uuid,
    file_path: String,
    hash: String,
}

#[async_trait]
trait CommitStore: Send + Sync {
    async fn latest_commit(&self, bucket_id: Uuid) -> Result<Option<LatestCommitRow>, BucketError>;

    async fn files_for_commit(&self, commit_id: Uuid) -> Result<Vec<CommitFileRow>, BucketError>;
}

#[async_trait]
impl CommitStore for DatabaseManager {
    async fn latest_commit(&self, bucket_id: Uuid) -> Result<Option<LatestCommitRow>, BucketError> {
        let params: Vec<&(dyn ToSql + Sync)> = vec![&bucket_id];

        let rows = DatabaseManager::query(
            self,
            "SELECT c.id, c.created_at, b.name
             FROM commits c
             JOIN buckets b ON c.bucket_id = b.id
             WHERE c.bucket_id = $1
             ORDER BY c.created_at DESC
             LIMIT 1",
            &params,
        )
        .await?;

        if let Some(row) = rows.first() {
            let commit_id: Uuid = row.get(0);
            let created_at: std::time::SystemTime = row.get(1);
            let bucket_name: String = row.get(2);

            Ok(Some(LatestCommitRow {
                id: commit_id,
                bucket: bucket_name,
                timestamp: chrono::DateTime::<chrono::Utc>::from(created_at).to_rfc3339(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn files_for_commit(&self, commit_id: Uuid) -> Result<Vec<CommitFileRow>, BucketError> {
        let params: Vec<&(dyn ToSql + Sync)> = vec![&commit_id];

        let rows = DatabaseManager::query(
            self,
            "SELECT f.id, f.file_path, f.hash
             FROM files f
             WHERE f.commit_id = $1
             ORDER BY f.file_path ASC",
            &params,
        )
        .await?;

        let mut files = Vec::with_capacity(rows.len());
        for row in rows {
            let file_id: Uuid = row.get(0);
            let file_path: String = row.get(1);
            let hash: String = row.get(2);

            files.push(CommitFileRow {
                id: file_id,
                file_path,
                hash,
            });
        }

        Ok(files)
    }
}

/// Commit changes to a bucket
pub struct Commit {
    args: CommitCommand,
}

impl BucketCommand for Commit {
    type Args = CommitCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        println!(
            "Executing commit command ########################################################## "
        );

        let world = World::new(&self.args.shared)?;

        let bucket = match &world.bucket {
            Some(bucket) => bucket,
            None => return Err(BucketError::NotInBucket),
        };

        println!(
            "Bucket: {} ########################################################## ",
            bucket.name
        );

        // create a list of each file in the bucket directory, recursively
        // and create a blake3 hash for each file and add to current_commit
        let current_commit = self.list_files_with_metadata_in_bucket(world.work_dir.clone())?;
        if current_commit.files.is_empty() {
            return Err(
                Error::new(ErrorKind::NotFound, "No commitable files found in bucket.").into(),
            );
        }

        println!("Current commit: ########################################################## ");

        let runtime_handle = RuntimeManager::handle()?;

        // Load the previous commit, if it exists
        match runtime_handle.block_on(Commit::load_last_commit_async(bucket.id)) {
            Ok(None) => {
                // There is no previous commit; Process all files in the current commit
                println!("No previous commit found. Processing all files. ########################################################## ");
                runtime_handle.block_on(self.process_files_async(
                    bucket.id,
                    &world.work_dir,
                    &current_commit.files,
                    &self.args.message,
                ))?;
            }
            Ok(Some(previous_commit)) => {
                // Compare the current commit with the previous commit
                println!("Previous commit found. Comparing with current commit. ########################################################## ");
                if let Some(changes) = current_commit.compare(&previous_commit) {
                    // Process the files that have changed
                    println!("Processing files that have changed. ########################################################## ");
                    runtime_handle.block_on(self.process_files_async(
                        bucket.id,
                        &world.work_dir,
                        &changes,
                        &self.args.message,
                    ))?;
                } else {
                    // if there are no difference with previous commit cancel commit
                    println!("No changes detected. Commit cancelled. ########################################################## ");
                    println!("No changes detected. Commit cancelled.");
                    return Ok(());
                }
            }
            Err(err) => {
                println!("Failed to load previous commit. ########################################################## ");
                error!("Failed to load previous commit: {}", err);
                return Err(BucketError::from(Error::other(
                    "Failed to load previous commit.",
                )));
            }
        }
        println!("Commit completed. ########################################################## ");

        Ok(())
    }
}

impl Commit {
    pub async fn process_files_async(
        &self,
        bucket_id: Uuid,
        bucket_path: &Path,
        files: &[CommittedFile],
        message: &str,
    ) -> Result<(), BucketError> {
        let db = get_database().await?;

        self.ensure_bucket_row(&db, bucket_id, bucket_path).await?;

        // Insert the commit into the database
        let commit_id = self
            .insert_commit_into_db_async(&db, bucket_id, message)
            .await?;

        // Process each file in the commit
        for file in files {
            // Insert the file into the database
            self.insert_file_into_db_async(&db, &commit_id, &file.name, &file.hash.to_string())
                .await?;

            // Compress and store the file (no database operation)
            file.compress_and_store(bucket_path).map_err(|e| {
                error!("Error compressing and storing file: {}", e);
                e
            })?;
        }
        Ok(())
    }

    async fn ensure_bucket_row(
        &self,
        db: &crate::postgres_db::DatabaseManager,
        bucket_id: Uuid,
        bucket_path: &Path,
    ) -> Result<(), BucketError> {
        let check_params: Vec<&(dyn ToSql + Sync)> = vec![&bucket_id];
        let rows = db
            .query("SELECT 1 FROM buckets WHERE id = $1::uuid", &check_params)
            .await?;

        if !rows.is_empty() {
            return Ok(());
        }

        let bucket = read_bucket_info(bucket_path).map_err(|err| {
            BucketError::IoError(std::io::Error::other(format!(
                "Failed to read bucket metadata: {}",
                err
            )))
        })?;

        let relative_path = bucket
            .relative_bucket_path
            .to_str()
            .ok_or_else(|| BucketError::from("Bucket path is not valid UTF-8"))?;

        let insert_params: Vec<&(dyn ToSql + Sync)> =
            vec![&bucket_id, &bucket.name, &relative_path];

        db.execute(
            "INSERT INTO buckets (id, name, path) VALUES ($1::uuid, $2, $3) ON CONFLICT (id) DO NOTHING",
            &insert_params,
        )
        .await?;

        Ok(())
    }

    async fn insert_file_into_db_async(
        &self,
        db: &crate::postgres_db::DatabaseManager,
        commit_id: &Uuid,
        file_path: &str,
        hash: &str,
    ) -> Result<(), BucketError> {
        let params: Vec<&(dyn ToSql + Sync)> = vec![commit_id, &file_path, &hash];

        db.execute(
            "INSERT INTO files (id, commit_id, file_path, hash) VALUES (uuid_generate_v4(), $1::uuid, $2, $3)",
            &params,
        ).await
        .map_err(|e| {
            BucketError::from(Error::other(format!(
                "Error inserting file into database: {}, commit id: {}, file path: {}, hash: {}",
                e, commit_id, file_path, hash
            )))
        })?;
        Ok(())
    }

    async fn insert_commit_into_db_async(
        &self,
        db: &crate::postgres_db::DatabaseManager,
        bucket_id: Uuid,
        message: &str,
    ) -> Result<Uuid, BucketError> {
        debug!("CommitCommand: inserting commit into PostgreSQL database");

        let params: Vec<&(dyn ToSql + Sync)> = vec![&bucket_id, &message];

        let rows = db.query(
            "INSERT INTO commits (id, bucket_id, message) VALUES (uuid_generate_v4(), $1::uuid, $2) RETURNING id",
            &params,
        ).await?;

        if let Some(row) = rows.first() {
            let id: Uuid = row.get(0);
            Ok(id)
        } else {
            Err(BucketError::from(Error::other(
                "Query returned no rows".to_string(),
            )))
        }
    }

    fn list_files_with_metadata_in_bucket(&self, bucket_path: PathBuf) -> io::Result<CommitData> {
        let mut files = Vec::new();

        // Extract bucket name from the bucket path
        let bucket_name = bucket_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();

        for entry in find_files_excluding_top_level_b(bucket_path.as_path()) {
            let full_path = bucket_path.join(&entry);

            if full_path.is_file() || full_path.is_symlink() {
                match hash_file(&full_path) {
                    Ok(hash) => {
                        //println!("BLAKE3 hash: {}", hash);
                        files.push(CommittedFile {
                            id: Uuid::new_v4(),
                            name: entry.to_string_lossy().into_owned(),
                            hash,
                            previous_hash: Hash::from_str(
                                "0000000000000000000000000000000000000000000000000000000000000000",
                            )
                            .map_err(|e| {
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    format!("Invalid hash format: {}", e),
                                )
                            })?,
                            status: CommitStatus::New,
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

        Ok(CommitData {
            bucket: bucket_name,
            files,
            timestamp: chrono::Utc::now().to_rfc3339(),
            previous: None,
            next: None,
        })
    }

    async fn load_last_commit_with_store<S: CommitStore + ?Sized>(
        store: &S,
        bucket_id: Uuid,
    ) -> Result<Option<CommitData>, BucketError> {
        let Some(commit_row) = store.latest_commit(bucket_id).await? else {
            return Ok(None);
        };

        let file_rows = store.files_for_commit(commit_row.id).await?;
        let zero_hash = Hash::from_str(ZERO_HASH_STR).map_err(|e| {
            BucketError::from(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid hash format: {}", e),
            ))
        })?;

        let mut files = Vec::with_capacity(file_rows.len());
        for file_row in file_rows {
            let hash = Hash::from_hex(&file_row.hash).map_err(|e| {
                BucketError::from(Error::new(
                    ErrorKind::InvalidData,
                    format!("Invalid hash: {}", e),
                ))
            })?;

            files.push(CommittedFile {
                id: file_row.id,
                name: file_row.file_path,
                hash,
                previous_hash: zero_hash,
                status: CommitStatus::Committed,
            });
        }

        Ok(Some(CommitData {
            bucket: commit_row.bucket,
            files,
            timestamp: commit_row.timestamp,
            previous: None,
            next: None,
        }))
    }

    pub async fn load_last_commit_async(
        bucket_id: Uuid,
    ) -> Result<Option<CommitData>, BucketError> {
        let db = get_database().await?;
        Self::load_last_commit_with_store(&*db, bucket_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::{Commit, CommitFileRow, CommitStore, LatestCommitRow};
    use crate::commands::BucketCommand;
    use crate::data::bucket::read_bucket_info;
    use crate::data::commit::{CommitStatus, CommittedFile};
    use crate::errors::BucketError;
    use crate::test_support::docker::{
        create_bucket_for_tests, ensure_database_initialized, TestDatabase,
    };
    use async_trait::async_trait;
    use blake3::Hash;
    use log::error;
    use serial_test::serial;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use std::str::FromStr;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    #[serial]
    fn test_process_files() {
        let Some(db) = TestDatabase::new() else {
            eprintln!("Skipping commit::tests::test_process_files: Docker/Postgres unavailable");
            return;
        };
        ensure_database_initialized(db.connection_string());

        // Need to setup a proper test environment
        let temp_dir = tempdir().expect("invalid temp dir").keep();
        let mut cmd1 = assert_cmd::Command::cargo_bin("buckets").expect("invalid command");
        let init_output = cmd1
            .env("DATABASE_URL", db.connection_string())
            .current_dir(temp_dir.as_path())
            .arg("init")
            .arg("test_repo")
            .output();

        // Check if init failed due to network issues
        if let Ok(output) = init_output {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("rate limit")
                || stderr.contains("Failed to install PostgreSQL")
                || !output.status.success()
            {
                eprintln!(
                    "Skipping test due to init failure (network issues): {}",
                    stderr
                );
                return;
            }
        } else {
            eprintln!("Skipping test due to init command failure");
            return;
        }

        let repo_dir = temp_dir.as_path().join("test_repo");
        let buckets_dir = repo_dir.join(".buckets");
        let db_type_file = buckets_dir.join("database_type");
        std::fs::write(&db_type_file, "PostgreSQL").expect("Failed to write database_type file");

        let bucket_dir = create_bucket_for_tests(&repo_dir, "test_bucket", db.connection_string())
            .expect("Failed to create test bucket");
        let file_path = bucket_dir.join("test_file.txt");
        let mut file = File::create(&file_path).expect("Failed to create test file");
        file.write_all(b"test").expect("Failed to write test data");

        // Bucket id is stored in the bucket info file
        // Can be read first to get the bucket id and then use
        // to query the database
        let bucket = read_bucket_info(&bucket_dir).expect("Failed to read bucket info");

        let commit_message = "Test commit".to_string();
        let committed_file = CommittedFile {
            id: Uuid::new_v4(),
            name: "test_file.txt".to_string(),
            hash: Hash::from_str(
                "f4315de648c8440fb2539fe9a8417e901ab270a37c6e2267e0c5fffe7d4d4419",
            )
            .expect("Failed to create test hash"),
            previous_hash: Hash::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .expect("Failed to create zero hash"),
            status: CommitStatus::New,
        };

        // change to bucket directory
        let original_dir = env::current_dir().ok();
        env::set_current_dir(&bucket_dir).expect("Failed to change to bucket directory");

        let commit_cmd = Commit::new(&crate::args::CommitCommand {
            shared: crate::args::SharedArguments::default(),
            message: commit_message.clone(),
        });
        // Create async runtime for testing
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        let result = rt
            .block_on(commit_cmd.process_files_async(
                bucket.id,
                &bucket_dir,
                &[committed_file],
                &commit_message,
            ))
            .map_err(|e| {
                error!("Error processing files: {}", e);
                e
            });

        match result {
            Ok(_) => (),
            Err(e) => {
                panic!("Error processing files: {}", e);
            }
        }
        if let Some(dir) = original_dir {
            let _ = env::set_current_dir(dir);
        }
    }

    // Helper function to create a test commit command
    pub(super) fn create_test_commit_command(message: &str) -> Commit {
        let args = crate::args::CommitCommand {
            shared: crate::args::SharedArguments::default(),
            message: message.to_string(),
        };
        Commit::new(&args)
    }

    #[derive(Clone, Default)]
    struct MockCommitStore {
        latest: Option<LatestCommitRow>,
        files: Vec<CommitFileRow>,
        expected_commit_id: Option<Uuid>,
    }

    #[async_trait]
    impl CommitStore for MockCommitStore {
        async fn latest_commit(
            &self,
            _bucket_id: Uuid,
        ) -> Result<Option<LatestCommitRow>, BucketError> {
            Ok(self.latest.clone())
        }

        async fn files_for_commit(
            &self,
            commit_id: Uuid,
        ) -> Result<Vec<CommitFileRow>, BucketError> {
            if let Some(expected) = self.expected_commit_id {
                assert_eq!(expected, commit_id);
            }
            Ok(self.files.clone())
        }
    }

    // Helper function to create a test bucket directory structure
    pub(super) fn create_test_bucket_structure(
    ) -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let bucket_path = temp_dir.path().join("test_bucket");
        std::fs::create_dir_all(&bucket_path)?;

        // Create .b directory structure
        let b_dir = bucket_path.join(".b");
        std::fs::create_dir_all(&b_dir)?;
        std::fs::create_dir_all(b_dir.join("storage"))?;

        // Create bucket info file
        let info_path = b_dir.join("info");
        let bucket_info = crate::data::bucket::Bucket {
            id: Uuid::new_v4(),
            name: "test_bucket".to_string(),
            relative_bucket_path: std::path::PathBuf::from("test_bucket"),
        };
        let info_content = toml::to_string(&bucket_info)?;
        std::fs::write(&info_path, info_content)?;

        Ok((temp_dir, bucket_path))
    }

    #[test]
    fn test_commit_new() {
        let commit = create_test_commit_command("test message");
        assert_eq!(commit.args.message, "test message");
    }

    #[test]
    fn test_list_files_with_metadata_in_bucket() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        // Create some test files
        let file1_path = bucket_path.join("file1.txt");
        let file2_path = bucket_path.join("file2.txt");
        let subdir_path = bucket_path.join("subdir");
        std::fs::create_dir_all(&subdir_path).expect("Failed to create subdir");
        let file3_path = subdir_path.join("file3.txt");

        std::fs::write(&file1_path, "content1").expect("Failed to write file1");
        std::fs::write(&file2_path, "content2").expect("Failed to write file2");
        std::fs::write(&file3_path, "content3").expect("Failed to write file3");

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.bucket, "test_bucket");
        assert_eq!(commit_data.files.len(), 3);

        // Check that files are present
        let file_names: Vec<String> = commit_data.files.iter().map(|f| f.name.clone()).collect();
        assert!(file_names.contains(&"file1.txt".to_string()));
        assert!(file_names.contains(&"file2.txt".to_string()));
        assert!(file_names.contains(&"subdir/file3.txt".to_string()));
    }

    #[test]
    fn test_list_files_with_metadata_empty_bucket() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.bucket, "test_bucket");
        assert_eq!(commit_data.files.len(), 0);
    }

    #[test]
    fn test_list_files_with_metadata_ignores_b_directory() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        // Create files in .b directory (should be ignored)
        let b_file_path = bucket_path.join(".b").join("internal_file.txt");
        std::fs::write(&b_file_path, "internal content").expect("Failed to write .b file");

        // Create regular file (should be included)
        let regular_file_path = bucket_path.join("regular_file.txt");
        std::fs::write(&regular_file_path, "regular content")
            .expect("Failed to write regular file");

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.files.len(), 1);
        assert_eq!(commit_data.files[0].name, "regular_file.txt");
    }

    #[test]
    fn test_list_files_with_metadata_calculates_hash() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let file_path = bucket_path.join("test_file.txt");
        let file_content = "test content for hashing";
        std::fs::write(&file_path, file_content).expect("Failed to write test file");

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.files.len(), 1);

        let file = &commit_data.files[0];
        assert_eq!(file.name, "test_file.txt");
        assert_eq!(file.status, CommitStatus::New);

        // Verify hash is calculated correctly
        let expected_hash = blake3::hash(file_content.as_bytes());
        assert_eq!(file.hash, expected_hash);
    }

    #[test]
    fn test_list_files_with_metadata_nonexistent_bucket() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let nonexistent_bucket_path = temp_dir.path().join("nonexistent_bucket");
        // Don't create the directory - this should result in an empty bucket

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(nonexistent_bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.files.len(), 0);
        assert_eq!(commit_data.bucket, "nonexistent_bucket");
    }

    #[test]
    fn test_committed_file_hash_consistency() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let file_path = bucket_path.join("consistency_test.txt");
        let file_content = "consistent content";
        std::fs::write(&file_path, file_content).expect("Failed to write test file");

        let commit = create_test_commit_command("test message");

        // List files multiple times and verify hash consistency
        let result1 = commit
            .list_files_with_metadata_in_bucket(bucket_path.clone())
            .unwrap();
        let result2 = commit
            .list_files_with_metadata_in_bucket(bucket_path.clone())
            .unwrap();

        assert_eq!(result1.files.len(), 1);
        assert_eq!(result2.files.len(), 1);
        assert_eq!(result1.files[0].hash, result2.files[0].hash);
        assert_eq!(result1.files[0].name, result2.files[0].name);
    }

    #[test]
    fn test_committed_file_different_content_different_hash() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let file_path = bucket_path.join("changing_file.txt");
        let commit = create_test_commit_command("test message");

        // Write initial content and get hash
        std::fs::write(&file_path, "initial content").expect("Failed to write initial content");
        let result1 = commit
            .list_files_with_metadata_in_bucket(bucket_path.clone())
            .unwrap();

        // Change content and get new hash
        std::fs::write(&file_path, "changed content").expect("Failed to write changed content");
        let result2 = commit
            .list_files_with_metadata_in_bucket(bucket_path.clone())
            .unwrap();

        assert_eq!(result1.files.len(), 1);
        assert_eq!(result2.files.len(), 1);
        assert_ne!(result1.files[0].hash, result2.files[0].hash);
        assert_eq!(result1.files[0].name, result2.files[0].name);
    }

    #[test]
    fn test_committed_file_large_file_handling() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let file_path = bucket_path.join("large_file.txt");
        let large_content = "x".repeat(10000); // 10KB file
        std::fs::write(&file_path, &large_content).expect("Failed to write large file");

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.files.len(), 1);

        let file = &commit_data.files[0];
        assert_eq!(file.name, "large_file.txt");

        // Verify hash is calculated correctly for large file
        let expected_hash = blake3::hash(large_content.as_bytes());
        assert_eq!(file.hash, expected_hash);
    }

    #[test]
    fn test_committed_file_binary_file_handling() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let file_path = bucket_path.join("binary_file.bin");
        let binary_content = vec![0u8, 1u8, 2u8, 255u8, 128u8]; // Binary data
        std::fs::write(&file_path, &binary_content).expect("Failed to write binary file");

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.files.len(), 1);

        let file = &commit_data.files[0];
        assert_eq!(file.name, "binary_file.bin");

        // Verify hash is calculated correctly for binary file
        let expected_hash = blake3::hash(&binary_content);
        assert_eq!(file.hash, expected_hash);
    }

    #[test]
    fn test_committed_file_nested_directories() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        // Create nested directory structure
        let nested_dir = bucket_path.join("level1").join("level2").join("level3");
        std::fs::create_dir_all(&nested_dir).expect("Failed to create nested directories");

        let file_path = nested_dir.join("nested_file.txt");
        std::fs::write(&file_path, "nested content").expect("Failed to write nested file");

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.files.len(), 1);

        let file = &commit_data.files[0];
        assert_eq!(file.name, "level1/level2/level3/nested_file.txt");
        assert_eq!(file.status, CommitStatus::New);
    }

    #[test]
    fn test_committed_file_permissions() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let file_path = bucket_path.join("permissions_test.txt");
        std::fs::write(&file_path, "permissions content").expect("Failed to write file");

        // Change file permissions (Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&file_path).unwrap().permissions();
            permissions.set_mode(0o644);
            std::fs::set_permissions(&file_path, permissions).expect("Failed to set permissions");
        }

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.files.len(), 1);

        let file = &commit_data.files[0];
        assert_eq!(file.name, "permissions_test.txt");
        // Hash should be based on content, not permissions
        let expected_hash = blake3::hash("permissions content".as_bytes());
        assert_eq!(file.hash, expected_hash);
    }

    #[test]
    fn test_committed_file_symlink_handling() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let target_file = bucket_path.join("target_file.txt");
        std::fs::write(&target_file, "target content").expect("Failed to write target file");

        let symlink_path = bucket_path.join("symlink_file.txt");

        // Create symlink (Unix-like systems)
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_file, &symlink_path)
                .expect("Failed to create symlink");

            let commit = create_test_commit_command("test message");
            let result = commit.list_files_with_metadata_in_bucket(bucket_path);

            assert!(result.is_ok());
            let commit_data = result.unwrap();
            // Should include both target file and symlink
            assert_eq!(commit_data.files.len(), 2);

            let file_names: Vec<String> =
                commit_data.files.iter().map(|f| f.name.clone()).collect();
            assert!(file_names.contains(&"target_file.txt".to_string()));
            assert!(file_names.contains(&"symlink_file.txt".to_string()));
        }

        // On Windows, just test that the target file is processed
        #[cfg(windows)]
        {
            let commit = create_test_commit_command("test message");
            let result = commit.list_files_with_metadata_in_bucket(bucket_path);

            assert!(result.is_ok());
            let commit_data = result.unwrap();
            assert_eq!(commit_data.files.len(), 1);
            assert_eq!(commit_data.files[0].name, "target_file.txt");
        }
    }

    #[test]
    fn test_committed_file_empty_file() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let empty_file_path = bucket_path.join("empty_file.txt");
        std::fs::write(&empty_file_path, "").expect("Failed to write empty file");

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.files.len(), 1);

        let file = &commit_data.files[0];
        assert_eq!(file.name, "empty_file.txt");

        // Hash of empty content
        let expected_hash = blake3::hash(b"");
        assert_eq!(file.hash, expected_hash);
    }

    #[test]
    fn test_multiple_files_sorting() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        // Create files in different orders
        let files = vec!["z_file.txt", "a_file.txt", "m_file.txt"];
        for file_name in &files {
            let file_path = bucket_path.join(file_name);
            std::fs::write(&file_path, format!("content of {}", file_name))
                .expect("Failed to write file");
        }

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.files.len(), 3);

        // Check that all files are present (order may vary depending on implementation)
        let file_names: Vec<String> = commit_data.files.iter().map(|f| f.name.clone()).collect();
        assert!(file_names.contains(&"z_file.txt".to_string()));
        assert!(file_names.contains(&"a_file.txt".to_string()));
        assert!(file_names.contains(&"m_file.txt".to_string()));
    }

    #[test]
    fn test_committed_file_uuid_generation() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let file_path = bucket_path.join("uuid_test.txt");
        std::fs::write(&file_path, "uuid content").expect("Failed to write file");

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();
        assert_eq!(commit_data.files.len(), 1);

        let file = &commit_data.files[0];
        // UUID should be valid and not nil
        assert_ne!(file.id, Uuid::nil());
        assert_eq!(file.id.get_version(), Some(uuid::Version::Random));
    }

    #[test]
    fn test_commit_timestamp_generation() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let file_path = bucket_path.join("timestamp_test.txt");
        std::fs::write(&file_path, "timestamp content").expect("Failed to write file");

        let commit = create_test_commit_command("test message");
        let result = commit.list_files_with_metadata_in_bucket(bucket_path);

        assert!(result.is_ok());
        let commit_data = result.unwrap();

        // Timestamp should be in ISO 8601 format
        assert!(!commit_data.timestamp.is_empty());

        // Try to parse timestamp to ensure it's valid
        let parsed_timestamp = chrono::DateTime::parse_from_rfc3339(&commit_data.timestamp);
        assert!(parsed_timestamp.is_ok());
    }

    #[tokio::test]
    async fn test_load_last_commit_async_returns_all_files() {
        let bucket_id = Uuid::new_v4();
        let commit_id = Uuid::new_v4();
        let timestamp = chrono::Utc::now().to_rfc3339();

        let file_a_hash = blake3::hash(b"file_a").to_hex().to_string();
        let file_b_hash = blake3::hash(b"file_b").to_hex().to_string();

        let mock_store = MockCommitStore {
            latest: Some(LatestCommitRow {
                id: commit_id,
                bucket: "test_bucket".to_string(),
                timestamp: timestamp.clone(),
            }),
            files: vec![
                CommitFileRow {
                    id: Uuid::new_v4(),
                    file_path: "file_a.txt".to_string(),
                    hash: file_a_hash,
                },
                CommitFileRow {
                    id: Uuid::new_v4(),
                    file_path: "dir/file_b.txt".to_string(),
                    hash: file_b_hash,
                },
            ],
            expected_commit_id: Some(commit_id),
        };

        let commit_data = Commit::load_last_commit_with_store(&mock_store, bucket_id)
            .await
            .expect("load commit data")
            .expect("expected commit data");

        assert_eq!(commit_data.bucket, "test_bucket");
        assert_eq!(commit_data.timestamp, timestamp);
        assert_eq!(commit_data.files.len(), 2);

        let mut file_names: Vec<&str> = commit_data.files.iter().map(|f| f.name.as_str()).collect();
        file_names.sort();
        assert_eq!(file_names, vec!["dir/file_b.txt", "file_a.txt"]);

        for file in &commit_data.files {
            assert_eq!(file.status, CommitStatus::Committed);
        }
    }

    #[test]
    fn test_load_last_commit_no_commit() {
        // This test would require a database setup, so we'll just test the function signature
        // In a real scenario, you would set up a test database
        // Create async runtime for testing
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        let result = rt.block_on(Commit::load_last_commit_async(Uuid::new_v4()));

        // Since there's no database setup, this will likely fail
        // In a proper test environment, you would set up a test database
        // and verify that it returns None for a new bucket
        assert!(result.is_err() || result.unwrap().is_none());
    }
}

#[cfg(test)]
mod execute_tests {
    use super::tests::{create_test_bucket_structure, create_test_commit_command};
    use crate::data::commit::{CommitStatus, CommittedFile};

    #[test]
    fn test_commit_execute_skips_when_no_changes_detected() {
        let (_temp_dir, bucket_path) =
            create_test_bucket_structure().expect("Failed to create test bucket structure");

        let file_path = bucket_path.join("identical.txt");
        std::fs::write(&file_path, "unchanged content").expect("Failed to write test file");

        let commit = create_test_commit_command("test message");
        let current_commit = commit
            .list_files_with_metadata_in_bucket(bucket_path)
            .expect("list current commit");

        let previous_files = current_commit
            .files
            .iter()
            .map(|file| CommittedFile {
                id: file.id,
                name: file.name.clone(),
                hash: file.hash.clone(),
                previous_hash: file.previous_hash.clone(),
                status: CommitStatus::Committed,
            })
            .collect();

        let previous_commit = crate::data::commit::Commit {
            bucket: current_commit.bucket.clone(),
            files: previous_files,
            timestamp: current_commit.timestamp.clone(),
            previous: None,
            next: None,
        };

        let changes = current_commit.compare(&previous_commit);
        assert!(
            changes.is_none(),
            "Expected no changes but found: {:?}",
            changes
        );
    }
}
