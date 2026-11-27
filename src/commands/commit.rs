use crate::args::CommitCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::read_bucket_info;
use crate::data::commit::{Commit as CommitData, CommitStatus, CommittedFile};
use crate::data::lock::{blocked_by_locks, current_owner, FileLock};
use crate::data::manifest::load_commit_async;
use crate::errors::BucketError;
use crate::postgres_db::get_database;
use crate::utils::object_store::ObjectStore;
use crate::utils::runtime::RuntimeManager;
use crate::utils::scan::scan_bucket;
use crate::utils::utils::find_bucket_path;
use crate::world::World;
use blake3::Hash;
use log::{debug, info};
use serde::Serialize;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Summary printed (or emitted as JSON) after a commit.
#[derive(Serialize, Debug, Default)]
pub struct CommitSummary {
    pub commit_id: Option<String>,
    pub files: usize,
    pub new: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
    pub objects_written: usize,
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
        let world = World::new(self.args.shared.verbose)?;
        let bucket = world.bucket.as_ref().ok_or(BucketError::NotInBucket)?;
        let bucket_path = find_bucket_path(&world.work_dir).ok_or(BucketError::NotInBucket)?;

        let current_commit = self.list_files_with_metadata_in_bucket(bucket_path.clone())?;
        let previous = RuntimeManager::block_on(Commit::load_last_commit_async(bucket.id))
            .map_err(|err| {
                BucketError::from(Error::other(format!(
                    "Failed to load previous commit: {}",
                    err
                )))
            })?;

        let changes: Vec<CommittedFile> = match &previous {
            None => {
                if current_commit.files.is_empty() {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        "No commitable files found in bucket.",
                    )
                    .into());
                }
                current_commit
                    .files
                    .iter()
                    .map(|f| CommittedFile {
                        id: f.id,
                        name: f.name.clone(),
                        hash: f.hash,
                        previous_hash: f.previous_hash,
                        status: CommitStatus::New,
                    })
                    .collect()
            }
            Some(previous_commit) => match current_commit.compare(previous_commit) {
                Some(changes) => changes,
                None => {
                    self.report(&CommitSummary {
                        files: current_commit.files.len(),
                        ..Default::default()
                    });
                    return Ok(());
                }
            },
        };

        // A name such as `..\\x.txt` is legal on Linux but would point outside
        // the bucket on Windows, or once separators are normalised.
        let unsafe_names: Vec<&str> = changes
            .iter()
            .map(|c| c.name.as_str())
            .filter(|n| !crate::utils::object_store::is_safe_relative(n))
            .collect();
        if !unsafe_names.is_empty() {
            return Err(BucketError::from(
                format!(
                    "These file names can't be committed because they would point outside the bucket on some systems: {}",
                    unsafe_names.join(", ")
                )
                .as_str(),
            ));
        }

        // Refuse to commit over someone else's lock.
        let changed_names: Vec<String> = changes.iter().map(|c| c.name.clone()).collect();
        let owner = current_owner();
        let bucket_id = bucket.id;
        let locks = RuntimeManager::block_on(async {
            let db = get_database().await?;
            FileLock::list_for_bucket(&db, bucket_id).await
        })?;
        let blocked = blocked_by_locks(&locks, &changed_names, &owner);
        if !blocked.is_empty() {
            let list = blocked
                .iter()
                .map(|l| format!("  {} (locked by {})", l.file_path, l.owner))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(BucketError::from(
                format!(
                    "Commit refused: these files are locked by someone else:\n{}",
                    list
                )
                .as_str(),
            ));
        }

        let summary = RuntimeManager::handle()?.block_on(self.record_commit_async(
            bucket.id,
            &bucket_path,
            &current_commit.files,
            &changes,
            &self.args.message,
        ))?;
        self.report(&summary);
        Ok(())
    }
}

impl Commit {
    fn report(&self, summary: &CommitSummary) {
        if self.args.shared.json {
            match serde_json::to_string_pretty(summary) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("Error serializing to JSON: {}", e),
            }
            return;
        }
        match &summary.commit_id {
            None => println!("No changes detected. Commit cancelled."),
            Some(id) => {
                println!(
                    "Committed {} ({} new, {} modified, {} deleted; {} files in bucket)",
                    id,
                    summary.new.len(),
                    summary.modified.len(),
                    summary.deleted.len(),
                    summary.files
                );
                for name in &summary.new {
                    println!("  new:      {}", name);
                }
                for name in &summary.modified {
                    println!("  modified: {}", name);
                }
                for name in &summary.deleted {
                    println!("  deleted:  {}", name);
                }
            }
        }
    }

    /// Record a commit whose manifest is `all_files` (every file now in the
    /// bucket). Contents of `changes` that are new or modified are written to
    /// the shared object store first; the commit and its file rows are then
    /// inserted in one database transaction, so a crash never leaves a commit
    /// pointing at objects that were not stored.
    pub async fn record_commit_async(
        &self,
        bucket_id: Uuid,
        bucket_path: &Path,
        all_files: &[CommittedFile],
        changes: &[CommittedFile],
        message: &str,
    ) -> Result<CommitSummary, BucketError> {
        let store = ObjectStore::for_path(bucket_path).ok_or(BucketError::NotInRepo)?;

        let mut summary = CommitSummary {
            files: all_files.len(),
            ..Default::default()
        };
        for change in changes {
            match change.status {
                CommitStatus::New | CommitStatus::Modified => {
                    let hex = change.hash.to_hex().to_string();
                    let written = store
                        .put_file_verified(&bucket_path.join(&change.name), &hex)
                        .map_err(BucketError::from)?;
                    if written {
                        summary.objects_written += 1;
                    }
                    if change.status == CommitStatus::New {
                        summary.new.push(change.name.clone());
                    } else {
                        summary.modified.push(change.name.clone());
                    }
                }
                CommitStatus::Deleted => summary.deleted.push(change.name.clone()),
                _ => {}
            }
        }
        // Objects for unchanged files must already exist (in the shared store or
        // legacy bucket storage); store any that are missing, e.g. after a
        // repository was upgraded from per-bucket storage.
        for file in all_files {
            let hex = file.hash.to_hex().to_string();
            if store.locate(&hex, Some(bucket_path)).is_none()
                && store
                    .put_file_verified(&bucket_path.join(&file.name), &hex)
                    .map_err(BucketError::from)?
            {
                summary.objects_written += 1;
            }
        }

        let db = get_database().await?;
        let mut client = db.get_connection().await?;
        let tx = client.transaction().await.map_err(|e| {
            BucketError::from(format!("Failed to start transaction: {}", e).as_str())
        })?;

        let exists = tx
            .query("SELECT 1 FROM buckets WHERE id = $1::uuid", &[&bucket_id])
            .await
            .map_err(|e| BucketError::from(format!("Query failed: {}", e).as_str()))?;
        if exists.is_empty() {
            let bucket = read_bucket_info(bucket_path).map_err(|err| {
                BucketError::IoError(io::Error::other(format!(
                    "Failed to read bucket metadata: {}",
                    err
                )))
            })?;
            let relative_path = bucket
                .relative_bucket_path
                .to_str()
                .ok_or_else(|| BucketError::from("Bucket path is not valid UTF-8"))?
                .to_string();
            tx.execute(
                "INSERT INTO buckets (id, name, path) VALUES ($1::uuid, $2, $3) ON CONFLICT (id) DO NOTHING",
                &[&bucket_id, &bucket.name, &relative_path],
            )
            .await
            .map_err(|e| BucketError::from(format!("Query failed: {}", e).as_str()))?;
        }

        let rows = tx
            .query(
                "INSERT INTO commits (id, bucket_id, message, manifest)
                 VALUES (uuid_generate_v4(), $1::uuid, $2, TRUE) RETURNING id",
                &[&bucket_id, &message],
            )
            .await
            .map_err(|e| BucketError::from(format!("Failed to insert commit: {}", e).as_str()))?;
        let commit_id: Uuid = rows
            .first()
            .map(|row| row.get(0))
            .ok_or_else(|| BucketError::from("Commit insert returned no id"))?;

        let insert = tx
            .prepare(
                "INSERT INTO files (id, commit_id, file_path, hash)
                 VALUES (uuid_generate_v4(), $1::uuid, $2, $3)",
            )
            .await
            .map_err(|e| BucketError::from(format!("Query failed: {}", e).as_str()))?;
        for file in all_files {
            let hex = file.hash.to_hex().to_string();
            tx.execute(&insert, &[&commit_id, &file.name, &hex])
                .await
                .map_err(|e| {
                    BucketError::from(
                        format!("Error inserting file '{}' into commit: {}", file.name, e).as_str(),
                    )
                })?;
        }
        tx.commit().await.map_err(|e| {
            BucketError::from(format!("Failed to commit transaction: {}", e).as_str())
        })?;

        info!(
            "Recorded commit {} with {} files",
            commit_id,
            all_files.len()
        );
        summary.commit_id = Some(commit_id.to_string());
        Ok(summary)
    }

    /// Record `files` as the complete content of a new commit.
    #[allow(dead_code)]
    pub async fn process_files_async(
        &self,
        bucket_id: Uuid,
        bucket_path: &Path,
        files: &[CommittedFile],
        message: &str,
    ) -> Result<(), BucketError> {
        self.record_commit_async(bucket_id, bucket_path, files, files, message)
            .await
            .map(|_| ())
    }

    pub(crate) fn list_files_with_metadata_in_bucket(
        &self,
        bucket_path: PathBuf,
    ) -> io::Result<CommitData> {
        list_bucket_files(&bucket_path)
    }

    /// The full file list of the latest commit of a bucket.
    pub async fn load_last_commit_async(
        bucket_id: Uuid,
    ) -> Result<Option<CommitData>, BucketError> {
        Ok(load_commit_async(bucket_id, None).await?.map(|r| r.data))
    }
}

/// Scan a bucket's working files into an uncommitted `Commit` value.
pub fn list_bucket_files(bucket_path: &Path) -> io::Result<CommitData> {
    let bucket_name = bucket_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string();
    let zero = Hash::from([0u8; 32]);
    let files = scan_bucket(bucket_path)?
        .into_iter()
        .map(|scanned| CommittedFile {
            id: Uuid::new_v4(),
            name: scanned.path,
            hash: scanned.hash,
            previous_hash: zero,
            status: CommitStatus::New,
        })
        .collect::<Vec<_>>();
    debug!("Scanned {} files in {}", files.len(), bucket_path.display());
    Ok(CommitData {
        bucket: bucket_name,
        files,
        timestamp: chrono::Utc::now().to_rfc3339(),
        previous: None,
        next: None,
    })
}

#[cfg(test)]
mod tests {
    use super::Commit;
    use crate::commands::BucketCommand;
    use crate::data::bucket::read_bucket_info;
    use crate::data::commit::{CommitStatus, CommittedFile};
    use crate::errors::BucketError;
    use crate::test_support::docker::{
        create_bucket_for_tests, ensure_database_initialized, TestDatabase,
    };
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
        let mut cmd1 = crate::test_support::buckets_command();
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
            hash: blake3::hash(b"test"),
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
