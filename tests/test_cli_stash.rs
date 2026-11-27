mod common;

#[cfg(test)]
mod tests {
    use crate::common::tests::RepoFixture;
    use serial_test::serial;

    /// Test the `stash` command.
    ///
    /// # Commands
    /// `$ buckets stash`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    fn test_cli_stash() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        // Stash works inside a bucket; with nothing changed it is a no-op.
        let bucket_dir = fixture.bucket_dir.clone();
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("stash")
            .assert()
            .success();

        // Outside a bucket it explains why it cannot run.
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(fixture.repo_dir.as_path())
            .arg("stash")
            .assert()
            .failure();
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI stash test: {message}");
                None
            }
        }
    }
}
