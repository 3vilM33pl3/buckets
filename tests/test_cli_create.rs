#[cfg(test)]
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;
    use predicates::prelude::predicate;

    /// Test the `create` command with no repository initialized.
    ///
    /// # Commands
    /// `$ buckets create test_bucket`
    ///
    /// # Expected output
    /// Error: No repository initialized.
    ///
    #[test]
    fn test_create_fail() {
        let temp_dir = tempdir().unwrap();
        let mut cmd_create = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_create.current_dir(temp_dir.path());
        cmd_create
            .arg("create")
            .arg("test_bucket")
            .assert()
            .failure();
    }

    /// Test the `create` command with a repository initialized.
    ///
    /// # Commands
    /// 1. `$ buckets init test_repo`
    /// 1. `$ buckets create test_bucket`
    ///
    /// # Expected output
    /// `test_bucket` directory created.
    /// `test_bucket/.b` directory created.
    /// `test_bucket/.b/storage` directory created.
    /// `test_bucket/.b/info` file created.
    ///
    /// Empty stdout.
    /// Empty stderr.
    ///
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

        let bucket_metadata_dir = bucket_dir.join(".b");
        assert!(bucket_metadata_dir.exists());
        assert!(bucket_metadata_dir.is_dir());

        let bucket_storage_dir = bucket_metadata_dir.join("storage");
        assert!(bucket_storage_dir.exists());
        assert!(bucket_storage_dir.is_dir());

        let bucket_info_file = bucket_metadata_dir.join("info");
        assert!(bucket_info_file.exists());
        assert!(bucket_info_file.is_file());



    }
}