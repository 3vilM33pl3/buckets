mod common;

#[cfg(test)]
mod tests {
    use crate::common::tests::RepoFixture;
    use serial_test::serial;

    /// Test the `link` command.
    ///
    /// # Commands
    /// `$ buckets link test_bucket`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    fn test_cli_link() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let temp_dir = fixture.repo_dir.clone();
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(temp_dir.as_path())
            .arg("link")
            .assert()
            .success();
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI link test: {message}");
                None
            }
        }
    }
}
