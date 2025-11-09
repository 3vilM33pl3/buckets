mod common;

#[cfg(test)]
mod acceptance_tests {
    use crate::common::tests::RepoFixture;
    use predicates::prelude::*;
    use serial_test::serial;
    use std::fs::File;
    use std::io::Write;

    /// Test the `history` command.
    ///
    /// # Commands
    /// `$ buckets history`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    fn test_cli_history_one_commit() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();

        create_test_file(&bucket_dir, "test_file.txt", "test content");

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(&bucket_dir)
            .arg("commit")
            .arg("test commit message")
            .assert()
            .success();

        // Test history command
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(&bucket_dir)
            .arg("history")
            .assert()
            .success()
            .stdout(predicate::str::contains("test commit message"))
            .stdout(predicate::str::contains("test_bucket"));
    }

    #[test]
    #[serial]
    fn test_cli_history_multiple_commits() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();

        create_test_file(&bucket_dir, "test_file.txt", "test content");
        create_test_file(&bucket_dir, "test_file2.txt", "test content 2");

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(&bucket_dir)
            .arg("commit")
            .arg("test commit message 1")
            .assert()
            .success();

        create_test_file(&bucket_dir, "test_file3.txt", "test content 3");

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(&bucket_dir)
            .arg("commit")
            .arg("test commit message 2")
            .assert()
            .success();

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(&bucket_dir)
            .arg("history")
            .assert()
            .success()
            .stdout(predicate::str::contains("test commit message 1"))
            .stdout(predicate::str::contains("test commit message 2"));
    }

    fn create_test_file(dir: &std::path::Path, filename: &str, content: &str) {
        let file_path = dir.join(filename);
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(content.as_bytes())
            .expect("Failed to write to file");
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI history test: {message}");
                None
            }
        }
    }
}
