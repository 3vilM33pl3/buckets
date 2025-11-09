mod common;

#[cfg(test)]
mod tests {
    use crate::common::tests::RepoFixture;
    use serial_test::serial;
    use std::{fs::File, io::Write};

    /// Test the `revert` command.
    ///
    /// # Commands
    /// `$ buckets revert`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    fn test_cli_revert() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let bucket_dir = fixture.bucket_dir.clone();
        let file_path = bucket_dir.join("test_file.txt");
        let mut file = File::create(&file_path).expect("invalid file");
        file.write_all(b"test").expect("invalid write");
        let mut cmd3 = assert_cmd::Command::cargo_bin("buckets").expect("invalid command");
        cmd3.current_dir(bucket_dir.as_path())
            .arg("commit")
            .arg("test message")
            .assert()
            .success();

        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("revert")
            .arg("test_file.txt")
            .assert()
            .success();
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI revert test: {message}");
                None
            }
        }
    }
}
