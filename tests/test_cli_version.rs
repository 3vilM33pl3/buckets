#[cfg(test)]
mod tests {
    #[test]
    fn test_cli_version() {
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd.assert().success();
        cmd.arg("version").assert().stdout("bucket version 0.1.0\n");
    }
}