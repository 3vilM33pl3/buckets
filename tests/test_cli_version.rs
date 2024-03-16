#[cfg(test)]
mod tests {
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