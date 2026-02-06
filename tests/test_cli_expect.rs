mod common;

#[cfg(test)]
mod tests {
    use crate::common::tests::RepoFixture;
    use predicates::prelude::*;
    use serial_test::serial;

    /// Test the `expect` command.
    ///
    /// # Commands
    /// `$ buckets expect`
    ///
    #[test]
    #[serial]
    fn test_cli_expect() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(fixture.bucket_dir.as_path())
            .arg("expect")
            .arg("Test Expectation")
            .assert()
            .success();
    }

    /// Test semantic duplicate detection warns when similar expectations exist
    /// Note: Requires model download on first run (~90MB)
    #[test]
    #[serial]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_cli_expect_duplicate_warning() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };

        // Create first expectation
        let mut cmd1 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd1.current_dir(fixture.bucket_dir.as_path())
            .arg("expect")
            .arg("API response time should be under 200ms")
            .assert()
            .success();

        // Create semantically similar expectation - should warn
        let mut cmd2 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd2.current_dir(fixture.bucket_dir.as_path())
            .arg("expect")
            .arg("The API must respond quickly within 200 milliseconds")
            .assert()
            .success()
            .stdout(predicate::str::contains("Similar expectations found"));
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(fixture) => Some(fixture),
            Err(message) => {
                eprintln!("Skipping CLI expect test: {message}");
                None
            }
        }
    }
}
