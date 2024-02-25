#[cfg(test)]
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_init() {
        let temp_dir = tempdir().unwrap();
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd.current_dir(temp_dir.path())
            .arg("init")
            .arg("test_repo")
            .assert()
            .success();

        let repo_dir = temp_dir.path().join("test_repo");

        assert!(repo_dir.exists());
        assert!(repo_dir.is_dir());
    }
}