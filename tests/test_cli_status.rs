mod common;
#[cfg(test)]
mod tests {
    use crate::common::tests::RepoFixture;
    use predicates::prelude::predicate;
    use serial_test::serial;
    use std::fs::File;
    use std::io::Write;

    /// Test the `status` command.
    ///
    /// # Commands
    /// `$ buckets status`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    fn test_cli_status() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let repo_dir = fixture.repo_dir.clone();
        let bucket_dir = fixture.bucket_dir.clone();
        let file_path = bucket_dir.join("test_file.txt");
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(b"test").expect("Failed to write to file");

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(repo_dir.as_path())
            .arg("status")
            .assert()
            .stdout(predicate::str::contains("Number of buckets: 1"))
            .success();

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("status")
            .assert()
            .stdout(predicate::str::contains("new:    test_file.txt"))
            .success();

        let mut cmd3 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd3.current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("test message")
            .assert()
            .success();

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("status")
            .assert()
            .stdout(predicate::str::contains("committed:    test_file.txt"))
            .success();
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI status test: {message}");
                None
            }
        }
    }
}
