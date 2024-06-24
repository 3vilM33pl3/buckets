#[cfg(test)]
mod tests {
    use predicates::prelude::predicate;
    use tempfile::tempdir;

    /// Test the `version` command.
    ///
    /// # Commands
    /// `$ buckets version`
    ///
    /// # Expected output
    /// bucket version 0.1.0
    ///
    #[test]
    fn test_cli_version() {
        let temp_dir = tempdir().unwrap();

        let mut cmd_init = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_init.current_dir(temp_dir.path());
        cmd_init
            .arg("init")
            .arg("test_repo")
            .assert()
            .success()
            .stdout(predicate::str::contains(""))
            .stderr(predicate::str::is_empty());

        let mut cmd_system = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_system.current_dir(temp_dir.path().join("test_repo"));
        cmd_system
            .arg("system")
            .assert()
            .stdout(predicate::str::contains("System Information:"))
            .success();
    }
}