#[cfg(test)]
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use super::*;

    /// Test the `commit` command with no files in the bucket.
    ///
    /// # Commands
    /// 1. `$ buckets init test_repo`
    /// 1. `$ buckets create test_bucket`
    /// 1. `$ buckets commit`
    ///
    /// # Expected output
    /// No files found in bucket. Commit cancelled.
    ///
    #[test]
    fn test_commit_no_files() {
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
        cmd_commit
            .arg("commit")
            .assert()
            .stdout("No files found in bucket. Commit cancelled.\n");
    }

    /// Test the `commit` command with one file in the bucket.
    ///
    /// # Commands
    /// 1. `$ buckets init test_repo`
    /// 1. `$ cd test_repo`
    /// 1. `$ buckets create test_bucket`
    /// 1. `$ echo "test" > test_bucket/test_file`
    /// 1. `$ buckets commit`
    ///
    /// # Expected output
    /// Commit successful.
    ///
    #[test]
    fn test_commit_one_files() {
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

        // write a single file
        let file_path = bucket_dir.join("test_file");
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"test").unwrap();

        cmd_commit.current_dir(bucket_dir);
        cmd_commit
            .arg("commit")
            .assert()
            .success();
    }

    #[test]
    fn test_commit_multiple_files() {
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

        // write a single file
        let file_path = bucket_dir.join("test_file");
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"test").unwrap();

        // write a second file
        let file_path = bucket_dir.join("test_file2");
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"test2").unwrap();

        cmd_commit.current_dir(bucket_dir);
        cmd_commit
            .arg("commit")
            .assert()
            .success();
    }

    #[test]
    fn test_commit_second_commit() {
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

        // write a single file
        let file_path = bucket_dir.join("test_file");
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"test").unwrap();

        cmd_commit.current_dir(&bucket_dir);
        cmd_commit
            .arg("commit")
            .assert()
            .success();

        let mut cmd_commit_2 = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_commit_2.current_dir(bucket_dir);
        cmd_commit_2
            .arg("commit")
            .assert()
            .stdout("No changes detected. Commit cancelled.\n");
    }

    #[test]
    fn test_commit_one_files_with_message() {
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

        // write a single file
        let file_path = bucket_dir.join("test_file");
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"test").unwrap();

        cmd_commit.current_dir(bucket_dir);
        cmd_commit
            .arg("commit")
            .arg("-m")
            .arg("test commit")
            .assert()
            .success();

        let message = get_message_from_database(repo_dir);

        assert_eq!(message, "test commit");
    }

    fn get_message_from_database(repo_dir: PathBuf) -> String {
        let db_location = repo_dir.join(".buckets/buckets.db");
        let conn = rusqlite::Connection::open(db_location).unwrap();

        let mut stmt = conn.prepare("SELECT message
                                               FROM commits
                                               WHERE bucket_id = (SELECT id FROM buckets WHERE name = 'test_bucket')").unwrap();

        let mut rows = stmt.query([]).unwrap();
        let row = rows.next().unwrap().unwrap();
        let message: String = row.get(0).unwrap();
        message
    }
}
