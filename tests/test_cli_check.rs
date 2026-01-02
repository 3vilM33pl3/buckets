mod common;

#[cfg(test)]
mod tests {
    use crate::common::tests::RepoFixture;
    use serial_test::serial;

    /// Test the `check` command.
    ///
    /// # Commands
    /// `$ typst check`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    fn test_cli_check() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let temp_dir = fixture.repo_dir.clone();
        // Create a bucket first
        let mut create_cmd =
            assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        create_cmd
            .current_dir(temp_dir.as_path())
            .arg("create")
            .arg("test_bucket")
            .assert()
            .success();

        // Run check on the created bucket
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(temp_dir.as_path())
            .arg("check")
            .arg("test_bucket")
            .assert()
            .success();
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI check test: {message}");
                None
            }
        }
    }
}
