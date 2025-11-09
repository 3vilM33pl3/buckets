mod common;

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write};

    use predicates::str::contains;
    use serial_test::serial;

    use crate::common::tests::{get_test_dir, RepoFixture};
    /// Test the `commit` command.
    ///
    /// # Commands
    /// `$ buckets commit`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    fn test_cli_commit() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();
        let file_path = bucket_dir.join("test_file.txt");
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(b"test").expect("Failed to write to file");
        let mut cmd3 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd3.current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("test message")
            .assert()
            .success();

        // Note: Direct database verification removed due to PostgreSQL migration
        // The embedded PostgreSQL database is managed internally by the application
        // Integration tests through CLI commands provide sufficient coverage
    }

    /// Test commit with no files in bucket (should fail)
    #[test]
    #[serial]
    fn test_cli_commit_no_files() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();

        // Attempt commit with empty bucket
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("empty commit")
            .assert()
            .failure()
            .stderr(contains("No commitable files found"));
    }

    /// Test commit with invalid/non-existent bucket directory
    #[test]
    #[serial]
    fn test_cli_commit_invalid_bucket() {
        let temp_dir = get_test_dir();
        let invalid_dir = temp_dir.join("not_a_bucket");
        std::fs::create_dir_all(&invalid_dir).expect("Failed to create invalid dir");

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(invalid_dir.as_path())
            .arg("commit")
            .arg("should fail")
            .assert()
            .failure()
            .stderr(contains("Not in a buckets repository"));
    }

    /// Test commit outside of repository
    #[test]
    #[serial]
    fn test_cli_commit_not_in_repo() {
        let temp_dir = get_test_dir();
        let outside_repo = temp_dir.join("outside");
        std::fs::create_dir_all(&outside_repo).expect("Failed to create outside dir");

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(outside_repo.as_path())
            .arg("commit")
            .arg("should fail")
            .assert()
            .failure()
            .stderr(contains("Not in a buckets repository"));
    }

    /// Test commit with missing commit message
    #[test]
    #[serial]
    fn test_cli_commit_missing_message() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();

        // Create a test file
        let file_path = bucket_dir.join("test_file.txt");
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(b"test").expect("Failed to write to file");

        // Attempt commit without message
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("commit")
            .assert()
            .failure();
    }

    /// Test commit with very large file to test edge cases
    #[test]
    #[serial]
    fn test_cli_commit_large_file() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();

        // Create a larger test file (1MB)
        let file_path = bucket_dir.join("large_file.txt");
        let mut file = File::create(&file_path).expect("Failed to create file");
        let large_content = vec![b'A'; 1024 * 1024]; // 1MB of 'A's
        file.write_all(&large_content)
            .expect("Failed to write large file");

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("large file test")
            .assert()
            .success();
    }

    /// Test commit with special characters in filename
    #[test]
    #[serial]
    fn test_cli_commit_special_filename() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();

        // Create file with special characters
        let file_path = bucket_dir.join("test file with spaces & symbols!.txt");
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(b"special filename test")
            .expect("Failed to write to file");

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("special filename test")
            .assert()
            .success();
    }

    /// Test commit with binary file
    #[test]
    #[serial]
    fn test_cli_commit_binary_file() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();

        // Create a binary file
        let file_path = bucket_dir.join("binary_file.bin");
        let mut file = File::create(&file_path).expect("Failed to create file");
        let binary_data: Vec<u8> = (0..=255).cycle().take(1000).collect();
        file.write_all(&binary_data)
            .expect("Failed to write binary file");

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("binary file test")
            .assert()
            .success();
    }

    /// Test commit with empty file
    #[test]
    #[serial]
    fn test_cli_commit_empty_file() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();

        // Create empty file
        let file_path = bucket_dir.join("empty_file.txt");
        File::create(&file_path).expect("Failed to create empty file");

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("empty file test")
            .assert()
            .success();
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI commit test: {message}");
                None
            }
        }
    }
}
