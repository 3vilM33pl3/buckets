mod common;

/// Test the `check` command.
///
/// # Commands
/// `$ typst check`
///
/// # Expected output
///
#[cfg(test)]
mod tests {
    use crate::common::tests::RepoFixture;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_version() {
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.arg("--version").assert().success();
    }

    #[test]
    #[serial]
    fn test_cli_check() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        // Must run from inside a bucket directory when no bucket name is provided
        let bucket_dir = fixture.bucket_dir.clone();
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(bucket_dir.as_path())
            .arg("check")
            .assert()
            .success();
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI check test (tests.rs): {message}");
                None
            }
        }
    }
}
