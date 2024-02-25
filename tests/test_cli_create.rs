#[cfg(test)]
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;
    use predicates::prelude::predicate;

    #[test]
    fn test_create_fail() {
        let temp_dir = tempdir().unwrap();
        let mut cmd_create = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_create.current_dir(temp_dir.path());
        cmd_create
            .arg("create")
            .arg("test_bucket")
            .assert()
            .success();
    }

    #[test]
    fn test_create() {
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
        let repo_dir = temp_dir.path().join("test_repo");

        assert!(repo_dir.exists());
        assert!(repo_dir.is_dir());

        let mut cmd_create = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_create.current_dir(temp_dir.path().join("test_repo"));
        cmd_create
            .arg("create")
            .arg("test_bucket")
            .assert()
            .success()
            .stdout(predicate::str::contains(""))
            .stderr(predicate::str::is_empty());
        let bucket_dir = repo_dir.join("test_bucket");

        assert!(bucket_dir.exists());
        assert!(bucket_dir.is_dir());
    }
}