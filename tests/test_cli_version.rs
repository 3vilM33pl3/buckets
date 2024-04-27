#[cfg(test)]
mod tests {
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
        let mut cmd_version = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_version
            .arg("version")
            .assert()
            .stdout("bucket version 0.1.0\n")
            .success();
    }
}