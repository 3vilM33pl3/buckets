use crate::args::RevertCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::{Bucket, BucketTrait};
use crate::errors::BucketError;
use crate::postgres_db::get_database;
use crate::utils::checks;
use crate::utils::utils::find_bucket_path;
use crate::CURRENT_DIR;
use log::{debug, error};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use tokio_postgres::types::ToSql;

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

        let file_path = self.args.file.clone();

        // Create async runtime for database operations
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| BucketError::from(format!("Failed to create async runtime: {}", e).as_str()))?;

        // Get the file's hash from the specified commit or the last commit
        let hash = rt.block_on(async {
            let db = get_database().await?;
            
            let relative_path = PathBuf::from(&file_path)
                .strip_prefix(&bucket_path)
                .unwrap_or(&PathBuf::from(&file_path))
                .to_string_lossy()
                .to_string();

            let bucket_id_str = bucket.id.to_string();
            
            let (query, params): (String, Vec<&(dyn ToSql + Sync)>) = match &self.args.commit_id {
                Some(commit_id) => {
                    // Query for specific commit ID
                    let query = "SELECT f.hash 
                        FROM files f
                        JOIN commits c ON f.commit_id = c.id
                        WHERE f.file_path = $1
                        AND c.id = $2
                        AND c.bucket_id = $3".to_string();
                    let params: Vec<&(dyn ToSql + Sync)> = vec![&relative_path, commit_id, &bucket_id_str];
                    (query, params)
                },
                None => {
                    // Query for latest commit (existing behavior)
                    let query = "SELECT f.hash 
                        FROM files f
                        JOIN commits c ON f.commit_id = c.id
                        WHERE f.file_path = $1
                        AND c.created_at = (
                            SELECT MAX(created_at) 
                            FROM commits 
                            WHERE bucket_id = $2
                        )".to_string();
                    let params: Vec<&(dyn ToSql + Sync)> = vec![&relative_path, &bucket_id_str];
                    (query, params)
                }
            };

            let rows = db.query(&query, &params).await?;

            match rows.first() {
                Some(row) => {
                    let hash: String = row.get(0);
                    Ok(hash)
                },
                None => match &self.args.commit_id {
                    Some(commit_id) => Err(BucketError::from(format!(
                        "File '{}' not found in commit '{}'", file_path, commit_id
                    ).as_str())),
                    None => Err(BucketError::FileNotFound(file_path.clone())),
                }
            }
        })?;

        // Construct paths
        let storage_path = bucket_path.join(".b").join("storage").join(&hash);
        let target_path = PathBuf::from(&file_path);

        debug!(
            "Reverting {} from {}",
            target_path.display(),
            storage_path.display()
        );

        // Decompress and copy the file from storage
        self.decompress_and_revert_file(&storage_path, &target_path)
            .map_err(|e| {
                error!("Failed to revert file: {}", e);
                BucketError::from(e)
            })?;

        match &self.args.commit_id {
            Some(commit_id) => println!("Reverted {} from commit {}", file_path, commit_id),
            None => println!("Reverted {} from latest commit", file_path),
        }
        Ok(())
    }
}

impl Revert {
    fn decompress_and_revert_file(
        &self,
        storage_path: &PathBuf,
        target_path: &PathBuf,
    ) -> std::io::Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open the compressed file
        let input_file = File::open(storage_path)?;
        let reader = BufReader::new(input_file);

        // Delete the target file if it exists
        if target_path.exists() {
            std::fs::remove_file(target_path)?;
        }

        // Create the output file
        let output_file = File::create(target_path)?;
        let writer = BufWriter::new(output_file);

        // Create a zstd decoder
        let mut decoder = zstd::Decoder::new(reader)?;

        // Copy data from decoder to output
        std::io::copy(&mut decoder, &mut BufWriter::new(writer))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::compression::{compress_file, DEFAULT_COMPRESSION_LEVEL};
    use serial_test::serial;

    use super::*;
    use std::io::Write;
    use std::{env, fs};
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn test_revert_command() {
        // Setup test environment
        let temp_dir = tempdir().expect("invalid temp dir").keep();
        log::debug!("temp_dir: {:?}", temp_dir);
        let mut cmd1 = assert_cmd::Command::cargo_bin("buckets").expect("invalid command");
        cmd1.current_dir(temp_dir.as_path())
            .arg("init")
            .arg("test_repo")
            .assert()
            .success();

        let mut cmd2 = assert_cmd::Command::cargo_bin("buckets").expect("invalid command");
        let repo_dir = temp_dir.as_path().join("test_repo");
        cmd2.current_dir(repo_dir.as_path())
            .arg("create")
            .arg("test_bucket")
            .assert()
            .success();

        let bucket_dir = repo_dir.join("test_bucket");
        let file_path = bucket_dir.join("test_file.txt");
        let mut file = File::create(&file_path).expect("invalid file");
        let original_content = b"original content";
        file.write_all(original_content).expect("invalid write");

        let mut cmd3 = assert_cmd::Command::cargo_bin("buckets").expect("invalid command");
        cmd3.current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("test message")
            .assert()
            .success();

        // Modify the file
        let modified_content = b"modified content";
        let mut file = File::create(&file_path).unwrap();
        file.write_all(modified_content).unwrap();

        // Change to bucket directory
        env::set_current_dir(&bucket_dir).expect("invalid directory");

        // Revert the file (use relative path)
        let revert_cmd = RevertCommand {
            file: "test_file.txt".to_string(),
            shared: Default::default(),
            commit_id: None,
        };
        let cmd = Revert::new(&revert_cmd);
        cmd.execute().unwrap();

        // Verify the file was reverted using binary comparison
        let reverted_content = fs::read(&file_path).expect("invalid read");
        assert_eq!(reverted_content, original_content);
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

        // Call the function we're testing
        let revert_cmd = Revert::new(&RevertCommand {
            shared: crate::args::SharedArguments::default(),
            file: "test".to_string(),
            commit_id: None,
        });
        revert_cmd
            .decompress_and_revert_file(&compressed_path, &reverted_path)
            .expect("Failed to decompress and revert file");

        // Read the reverted content
        let reverted_content = std::fs::read(&reverted_path).expect("Failed to read reverted file");

        // Compare content
        assert_eq!(
            reverted_content, original_content,
            "Reverted content doesn't match original"
        );
    }
}
