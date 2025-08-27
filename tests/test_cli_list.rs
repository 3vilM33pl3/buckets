mod common;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::common::tests::get_test_dir;
    use serial_test::serial;

    /// Test the `list` command.
    ///
    /// # Commands
    /// `$ buckets list`
    ///
    /// # Expected output
    ///
    #[test]
    #[serial]
    #[ignore]
    fn test_cli_list() {
        let temp_dir = setup();
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(temp_dir.as_path())
            .arg("list")
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
