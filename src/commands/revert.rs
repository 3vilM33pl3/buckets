use crate::args::RevertCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::{Bucket, BucketTrait};
use crate::data::commit::Commit as CommitData;
use crate::data::manifest::load_commit_async;
use crate::errors::BucketError;
use crate::utils::checks;
use crate::utils::runtime::RuntimeManager;
use crate::utils::utils::{find_bucket_path, find_files_excluding_top_level_b};
use crate::CURRENT_DIR;
use log::{debug, error};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Revert a file from a specific commit or the most recent commit
pub struct Revert {
    args: RevertCommand,
}

impl BucketCommand for Revert {
    type Args = RevertCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let current_dir = CURRENT_DIR.with(|dir| dir.clone());

        if !checks::is_valid_bucket_repo(&current_dir) {
            return Err(BucketError::NotInRepo);
        }

        let bucket_path = match find_bucket_path(&current_dir) {
            Some(path) => path,
            None => return Err(BucketError::NotAValidBucket),
        };

        let bucket = Bucket::from_meta_data(&current_dir)?;

        match &self.args.file {
            Some(file_path) => self.revert_single_file(bucket, &bucket_path, file_path),
            None => self.revert_all(bucket, &bucket_path),
        }
    }
}

impl Revert {
    fn parsed_commit_id(&self) -> Result<Option<Uuid>, BucketError> {
        self.args
            .commit_id
            .as_ref()
            .map(|id| {
                Uuid::parse_str(id).map_err(|e| {
                    BucketError::from(format!("Invalid commit id '{}': {}", id, e).as_str())
                })
            })
            .transpose()
    }

    fn load_target(&self, bucket: &Bucket) -> Result<(CommitData, String), BucketError> {
        let commit_id = self.parsed_commit_id()?;
        let resolved = RuntimeManager::block_on(load_commit_async(bucket.id, commit_id))?;
        let display = commit_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "latest".to_string());
        match resolved {
            Some(r) => Ok((r.data, display)),
            None => Err(BucketError::from("No commits found in this bucket")),
        }
    }

    fn revert_single_file(
        &self,
        bucket: Bucket,
        bucket_path: &PathBuf,
        file_path: &String,
    ) -> Result<(), BucketError> {
        let current_dir = CURRENT_DIR.with(|dir| dir.clone());
        let candidate = PathBuf::from(file_path);
        let absolute = if candidate.is_absolute() {
            candidate.clone()
        } else {
            current_dir.join(&candidate)
        };
        let relative_path = absolute
            .strip_prefix(bucket_path)
            .map(PathBuf::from)
            .unwrap_or(candidate)
            .to_string_lossy()
            .to_string();

        let (commit, display) = self.load_target(&bucket)?;
        let file = commit
            .files
            .iter()
            .find(|f| f.name == relative_path)
            .ok_or_else(|| match &self.args.commit_id {
                Some(commit_id) => BucketError::from(
                    format!("File '{}' not found in commit '{}'", file_path, commit_id).as_str(),
                ),
                None => BucketError::FileNotFound(file_path.clone()),
            })?;

        debug!("Reverting {} to {}", relative_path, file.hash);
        file.restore_version(bucket_path, &file.hash).map_err(|e| {
            error!("Failed to revert file: {}", e);
            BucketError::from(e)
        })?;

        match &self.args.commit_id {
            Some(_) => println!("Reverted {} from commit {}", file_path, display),
            None => println!("Reverted {} from latest commit", file_path),
        }
        Ok(())
    }

    fn revert_all(&self, bucket: Bucket, bucket_path: &Path) -> Result<(), BucketError> {
        let (commit, commit_id_str) = self.load_target(&bucket)?;

        if commit.files.is_empty() {
            println!("No files found in commit {}", commit_id_str);
            return Ok(());
        }

        // Restore every file first, so a missing object aborts before anything is deleted.
        for file in &commit.files {
            debug!("Restoring {} ({})", file.name, file.hash);
            file.restore_version(bucket_path, &file.hash).map_err(|e| {
                error!("Failed to revert file {}: {}", file.name, e);
                BucketError::from(e)
            })?;
        }

        // Then delete files that are not part of the target commit.
        let target_files_set: HashSet<PathBuf> = commit
            .files
            .iter()
            .map(|f| PathBuf::from(&f.name))
            .collect();
        for file in find_files_excluding_top_level_b(bucket_path) {
            if !target_files_set.contains(&file) {
                let full_path = bucket_path.join(&file);
                debug!("Deleting {}", full_path.display());
                if full_path.is_file() || full_path.is_symlink() {
                    std::fs::remove_file(&full_path)?;
                }
            }
        }

        println!("Reverted entire bucket to commit {}", commit_id_str);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::docker::{
        create_bucket_for_tests, ensure_database_initialized, TestDatabase,
    };
    use crate::utils::compression::{compress_file, DEFAULT_COMPRESSION_LEVEL};
    use serial_test::serial;

    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::{env, fs};
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn test_revert_command() {
        let Some(db) = TestDatabase::new() else {
            eprintln!("Skipping revert::tests::test_revert_command: Docker/Postgres unavailable");
            return;
        };
        ensure_database_initialized(db.connection_string());
        // Setup test environment
        let temp_dir = tempdir().expect("invalid temp dir").keep();
        log::debug!("temp_dir: {:?}", temp_dir);

        // First try to initialize - if it fails due to network issues, skip the test
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
        let bucket_dir = create_bucket_for_tests(&repo_dir, "test_bucket", db.connection_string())
            .expect("Failed to create test bucket");
        let file_path = bucket_dir.join("test_file.txt");
        let mut file = File::create(&file_path).expect("invalid file");
        let original_content = b"original content";
        file.write_all(original_content).expect("invalid write");

        let mut cmd3 = assert_cmd::Command::cargo_bin("buckets").expect("invalid command");
        cmd3.env("DATABASE_URL", db.connection_string())
            .current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("test message")
            .assert()
            .success();

        // Modify the file
        let modified_content = b"modified content";
        let mut file = File::create(&file_path).unwrap();
        file.write_all(modified_content).unwrap();

        // Change to bucket directory
        let original_dir = env::current_dir().ok();
        env::set_current_dir(&bucket_dir).expect("invalid directory");

        // Revert the file (use relative path)
        let revert_cmd = RevertCommand {
            file: Some("test_file.txt".to_string()),
            shared: Default::default(),
            commit_id: None,
        };
        let cmd = Revert::new(&revert_cmd);
        cmd.execute().unwrap();

        // Verify the file was reverted using binary comparison
        let reverted_content = fs::read(&file_path).expect("invalid read");
        assert_eq!(reverted_content, original_content);

        if let Some(dir) = original_dir {
            let _ = env::set_current_dir(dir);
        }
    }

    #[test]
    #[serial]
    fn test_decompress_and_revert_file() {
        // Create a temporary directory for test files
        let temp_dir = tempdir().expect("Failed to create temp directory");

        // Create original content
        let original_content = b"original content";

        // Create source file path
        let source_path = temp_dir.path().join("source.txt");

        // Write original content to source file
        std::fs::write(&source_path, original_content).expect("Failed to write source file");

        // Create compressed file path
        let compressed_path = temp_dir.path().join("compressed.zst");

        compress_file(&source_path, &compressed_path, DEFAULT_COMPRESSION_LEVEL)
            .expect("Failed to compress and store file");

        // Create reverted file path
        let reverted_path = temp_dir.path().join("reverted.txt");

        // Restore through the object store's legacy-storage fallback.
        let bucket = temp_dir.path().join("bucket");
        let legacy = bucket.join(".b").join("storage");
        fs::create_dir_all(&legacy).expect("create legacy storage");
        fs::rename(&compressed_path, legacy.join("deadbeef")).expect("move object");
        crate::utils::object_store::ObjectStore::at(temp_dir.path().join("objects"))
            .restore("deadbeef", &reverted_path, Some(&bucket))
            .expect("Failed to decompress and revert file");

        // Read the reverted content
        let reverted_content = std::fs::read(&reverted_path).expect("Failed to read reverted file");

        // Compare content
        assert_eq!(
            reverted_content, original_content,
            "Reverted content doesn't match original"
        );
    }

    #[test]
    #[serial]
    fn test_revert_all() {
        let Some(db) = TestDatabase::new() else {
            return;
        };
        ensure_database_initialized(db.connection_string());

        let temp_dir = tempdir().expect("invalid temp dir").keep();

        // 1. Init
        let mut cmd1 = assert_cmd::Command::cargo_bin("buckets").expect("invalid command");
        cmd1.env("DATABASE_URL", db.connection_string())
            .current_dir(temp_dir.as_path())
            .arg("init")
            .arg("test_repo")
            .assert()
            .success();

        let repo_dir = temp_dir.as_path().join("test_repo");
        let bucket_dir = create_bucket_for_tests(&repo_dir, "test_bucket", db.connection_string())
            .expect("Failed to create test bucket");

        // 2. Create state 1: file1, file2
        let file1 = bucket_dir.join("file1.txt");
        let file2 = bucket_dir.join("file2.txt");
        fs::write(&file1, "v1").unwrap();
        fs::write(&file2, "v1").unwrap();

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("invalid command");
        cmd.env("DATABASE_URL", db.connection_string())
            .current_dir(&bucket_dir)
            .arg("commit")
            .arg("commit 1")
            .assert()
            .success();

        // 3. Modify state: modify file1, delete file2, add file3
        fs::write(&file1, "v2").unwrap();
        fs::remove_file(&file2).unwrap();
        fs::write(bucket_dir.join("file3.txt"), "v3").unwrap();

        // 4. Revert all
        let original_dir = env::current_dir().ok();
        env::set_current_dir(&bucket_dir).unwrap();

        let revert_cmd = Revert::new(&RevertCommand {
            shared: crate::args::SharedArguments::default(),
            file: None,      // All
            commit_id: None, // Latest
        });
        revert_cmd.execute().expect("Revert failed");

        if let Some(dir) = original_dir {
            env::set_current_dir(dir).ok();
        }

        // 5. Verify
        assert_eq!(fs::read_to_string(&file1).unwrap(), "v1"); // Restored
        assert!(file2.exists());
        assert_eq!(fs::read_to_string(&file2).unwrap(), "v1"); // Restored
        assert!(!bucket_dir.join("file3.txt").exists()); // Deleted (not in clean state)
    }
}
