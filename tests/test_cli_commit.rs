#[cfg(test)]
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit() {
        let temp_dir = tempdir().unwrap();

        let mut cmd_init = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_init.current_dir(temp_dir.path());
        cmd_init.arg("init").arg("test_repo").assert().success();
        let repo_dir = temp_dir.path().join("test_repo");

        let mut cmd_create = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_create.current_dir(repo_dir.clone());
        cmd_create
            .arg("create")
            .arg("test_bucket")
            .assert()
            .success();
        let bucket_dir = repo_dir.join("test_bucket");

        let mut cmd_commit = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_commit.current_dir(bucket_dir);
        cmd_commit.arg("commit").assert().success();
    }
}
