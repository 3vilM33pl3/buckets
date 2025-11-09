mod common;

#[cfg(test)]
mod tests {
    use crate::common::tests::RepoFixture;
    use serial_test::serial;

    /// Test the `finalize` command.
    ///
    /// # Commands
    /// `$ buckets finalize`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    fn test_cli_finalize() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let temp_dir = fixture.repo_dir.clone();
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(temp_dir.as_path())
            .arg("finalize")
            .assert()
            .success();
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI finalize test: {message}");
                None
            }
        }
    }
}
