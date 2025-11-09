mod common;

#[cfg(test)]
mod tests {
    use crate::common::tests::RepoFixture;
    use predicates::prelude::predicate;
    use serial_test::serial;
    use std::fs::File;
    use std::io::Write;

    /// Test the `rollback` command.
    ///
    /// # Commands
    /// `$ buckets rollback`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    fn test_cli_rollback() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();

        let file_path = bucket_dir.join("test_file.txt");

        // Create and write initial content
        {
            let mut file_1 = File::create(&file_path).expect("Failed to create file");
            file_1
                .write_all(b"test file 1")
                .expect("Failed to write to file");
        }

        let mut cmd1 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd1.current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("test message")
            .assert()
            .success();

        // Modify the file after the commit
        {
            let mut file_1 =
                File::create(&file_path).expect("Failed to create file for modification");
            file_1
                .write_all(b"change file 1")
                .expect("Failed to write to file");
        }

        let mut cmd2 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd2.current_dir(bucket_dir.as_path())
            .arg("status")
            .assert()
            .stdout(predicate::str::contains("modified:    test_file.txt"))
            .success();

        let mut cmd3 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd3.current_dir(bucket_dir.as_path())
            .arg("rollback")
            .assert()
            .success();

        let mut cmd4 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd4.current_dir(bucket_dir.as_path())
            .arg("status")
            .assert()
            .stdout(predicate::str::contains("committed:    test_file.txt"))
            .success();
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI rollback test: {message}");
                None
            }
        }
    }
}
