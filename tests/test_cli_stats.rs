mod common;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serial_test::serial;

    use crate::common::tests::get_test_dir;

    /// Test the `stats` command.
    ///
    /// # Commands
    /// `$ typst stats`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    #[ignore]
    fn test_cli_stats() {
        let temp_dir = setup();
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(temp_dir.as_path())
            .arg("stats")
            .assert()
            .success();
    }

    fn setup() -> PathBuf {
        let temp_dir = get_test_dir();
        let mut cmd1 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd1.current_dir(temp_dir.as_path())
            .arg("init")
            .arg("test_repo")
            .assert()
            .success();

        let mut cmd2 = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        let repo_dir = temp_dir.as_path().join("test_repo");
        cmd2.current_dir(repo_dir.as_path())
            .arg("create")
            .arg("test_bucket")
            .assert()
            .success();

        repo_dir
    }
}
