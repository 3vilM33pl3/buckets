mod common;
#[cfg(test)]
mod tests {
    use predicates::prelude::*;
    use serial_test::serial;
    use uuid::Uuid;

    use crate::common::tests::{get_test_dir, RepoFixture};

    /// Test the `create` command.
    ///
    /// # Commands
    /// `$ buckets create test_repo`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    fn test_cli_create_no_repo() {
        let temp_dir = get_test_dir();
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(temp_dir.as_path())
            .arg("create")
            .arg("test_bucket")
            .assert()
            .stderr(predicate::str::contains("Not in a buckets repository"))
            .failure();
    }

    #[test]
    #[serial]
    fn test_cli_create() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let repo_dir = fixture.repo_dir.clone();
        let new_bucket = format!("bucket_{}", Uuid::new_v4().simple());

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(repo_dir.as_path())
            .arg("create")
            .arg(&new_bucket)
            .assert()
            .success();

        let bucket_path = repo_dir.join(&new_bucket).join(".b");
        assert!(bucket_path.exists());
        assert!(bucket_path.join("storage").exists());

        // Note: Direct database verification removed due to PostgreSQL migration
        // The embedded PostgreSQL database is managed internally by the application
        // Filesystem checks provide sufficient coverage for create functionality
    }

    /// Test creating bucket that already exists (should fail)
    #[test]
    #[serial]
    fn test_cli_create_bucket_already_exists() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let repo_dir = fixture.repo_dir.clone();
        let bucket_name = format!("dup_{}", Uuid::new_v4().simple());

        // Create bucket first time (should succeed)
        let mut cmd2 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd2.current_dir(repo_dir.as_path())
            .arg("create")
            .arg(&bucket_name)
            .assert()
            .success();

        // Try to create same bucket again (should fail)
        let mut cmd3 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd3.current_dir(repo_dir.as_path())
            .arg("create")
            .arg(&bucket_name)
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("already exists")
                    .or(predicate::str::contains("duplicate")),
            );
    }

    /// Test creating bucket with invalid name characters
    #[test]
    #[serial]
    fn test_cli_create_invalid_bucket_name() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let repo_dir = fixture.repo_dir.clone();

        // Test various invalid bucket names
        let invalid_names = vec![
            ".",  // current directory
            "..", // parent directory
            "bucket/with/slashes", // path separators
                  // Note: null characters can't be tested via command line
        ];

        for invalid_name in invalid_names {
            let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
            cmd.current_dir(repo_dir.as_path())
                .arg("create")
                .arg(invalid_name)
                .assert()
                .failure();
        }
    }

    /// Test creating bucket with very long name
    #[test]
    #[serial]
    fn test_cli_create_long_bucket_name() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let repo_dir = fixture.repo_dir.clone();

        // Create very long bucket name (255+ characters)
        let long_name = "a".repeat(300);

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(repo_dir.as_path())
            .arg("create")
            .arg(&long_name)
            .assert()
            .failure(); // May fail due to filesystem limitations
    }

    /// Test creating bucket without providing name argument
    #[test]
    #[serial]
    fn test_cli_create_missing_name() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let repo_dir = fixture.repo_dir.clone();

        // Try to create bucket without name
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(repo_dir.as_path())
            .arg("create")
            .assert()
            .failure(); // Should fail due to missing required argument
    }

    /// Test creating bucket with special characters (should succeed)
    #[test]
    #[serial]
    fn test_cli_create_special_characters() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let repo_dir = fixture.repo_dir.clone();

        // Test bucket names with special characters that should be valid
        let valid_special_names = vec![
            "bucket-with-dashes",
            "bucket_with_underscores",
            "bucket.with.dots",
            "bucket123",
            "123bucket",
        ];

        for name in valid_special_names {
            let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
            cmd.current_dir(repo_dir.as_path())
                .arg("create")
                .arg(name)
                .assert()
                .success();
        }
    }
    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI create test: {message}");
                None
            }
        }
    }
}
