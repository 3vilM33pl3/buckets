#![allow(clippy::unwrap_used)]

mod common;

/// End-to-end checks for commit history, rollback, revert, locks, stash and
/// push/pull, run against a real PostgreSQL container.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    use serde_json::Value;
    use serial_test::serial;

    use crate::common::tests::RepoFixture;

    fn fixture_or_skip() -> Option<RepoFixture> {
        match RepoFixture::new() {
            Ok(f) => Some(f),
            Err(message) => {
                eprintln!("Skipping history integrity test: {message}");
                None
            }
        }
    }

    fn buckets(dir: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("binary");
        cmd.current_dir(dir).env("BUCKETS_USER", "tester");
        for a in args {
            cmd.arg(a);
        }
        cmd.assert()
    }

    fn json(dir: &Path, args: &[&str]) -> Value {
        let out = buckets(dir, args).success().get_output().stdout.clone();
        serde_json::from_slice(&out).unwrap_or_else(|e| {
            panic!(
                "invalid JSON from {:?}: {}\n{}",
                args,
                e,
                String::from_utf8_lossy(&out)
            )
        })
    }

    /// Map of file name to status from `buckets status --json`.
    fn status(dir: &Path) -> BTreeMap<String, String> {
        json(dir, &["status", "--json"])["files"]
            .as_array()
            .expect("files array")
            .iter()
            .map(|f| {
                (
                    f["name"].as_str().unwrap_or_default().to_string(),
                    f["status"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect()
    }

    fn all_committed(dir: &Path, names: &[&str]) -> bool {
        let s = status(dir);
        s.len() == names.len()
            && names
                .iter()
                .all(|n| s.get(*n).map(String::as_str) == Some("committed"))
    }

    #[test]
    #[serial]
    fn unchanged_and_deleted_files_are_tracked_across_commits() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::write(b.join("a.txt"), "a1").unwrap();
        fs::write(b.join("b.txt"), "b1").unwrap();
        fs::write(b.join("c.txt"), "c1").unwrap();
        buckets(&b, &["commit", "one"]).success();
        assert!(all_committed(&b, &["a.txt", "b.txt", "c.txt"]));

        // Edit one file: the others must still read as committed afterwards.
        fs::write(b.join("a.txt"), "a2").unwrap();
        assert_eq!(
            status(&b).get("a.txt").map(String::as_str),
            Some("modified")
        );
        let summary = json(&b, &["commit", "two", "--json"]);
        assert_eq!(summary["modified"], serde_json::json!(["a.txt"]));
        assert!(all_committed(&b, &["a.txt", "b.txt", "c.txt"]));

        // Delete a file: the deletion is recorded.
        fs::remove_file(b.join("c.txt")).unwrap();
        assert_eq!(status(&b).get("c.txt").map(String::as_str), Some("deleted"));
        let summary = json(&b, &["commit", "three", "--json"]);
        assert_eq!(summary["deleted"], serde_json::json!(["c.txt"]));
        assert!(all_committed(&b, &["a.txt", "b.txt"]));

        // Nothing changed: no new commit.
        let summary = json(&b, &["commit", "noop", "--json"]);
        assert!(summary["commit_id"].is_null());

        // Identical content is stored once in the shared store.
        let objects = fx.repo_dir.join(".buckets").join("objects");
        let count = walkdir::WalkDir::new(&objects)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .count();
        assert_eq!(count, 4, "a1, a2, b1, c1");
    }

    #[test]
    #[serial]
    fn rollback_and_revert_restore_committed_versions() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::write(b.join("model.blend"), "v1").unwrap();
        fs::write(b.join("tex.png"), "t1").unwrap();
        let first = json(&b, &["commit", "v1", "--json"])["commit_id"]
            .as_str()
            .unwrap()
            .to_string();
        fs::write(b.join("model.blend"), "v2").unwrap();
        buckets(&b, &["commit", "v2"]).success();

        // Single-file rollback of an edited file.
        fs::write(b.join("model.blend"), "scratch").unwrap();
        buckets(&b, &["rollback", "--path", "model.blend"]).success();
        assert_eq!(fs::read_to_string(b.join("model.blend")).unwrap(), "v2");

        // Single-file rollback of a deleted file.
        fs::remove_file(b.join("tex.png")).unwrap();
        buckets(&b, &["rollback", "--path", "tex.png"]).success();
        assert_eq!(fs::read_to_string(b.join("tex.png")).unwrap(), "t1");

        // Rollback of everything brings back deleted files too.
        fs::remove_file(b.join("tex.png")).unwrap();
        fs::write(b.join("model.blend"), "scratch").unwrap();
        buckets(&b, &["rollback"]).success();
        assert_eq!(fs::read_to_string(b.join("tex.png")).unwrap(), "t1");
        assert_eq!(fs::read_to_string(b.join("model.blend")).unwrap(), "v2");

        // Revert to the first commit restores every file it held, not just changed ones.
        fs::remove_file(b.join("tex.png")).unwrap();
        buckets(&b, &["revert", "--commit", &first]).success();
        assert_eq!(fs::read_to_string(b.join("model.blend")).unwrap(), "v1");
        assert_eq!(fs::read_to_string(b.join("tex.png")).unwrap(), "t1");
    }

    #[test]
    #[serial]
    fn locks_block_other_owners_from_committing() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::write(b.join("L_Island.umap"), "m1").unwrap();
        buckets(&b, &["commit", "map"]).success();

        let mut lock = assert_cmd::Command::cargo_bin("buckets").unwrap();
        lock.current_dir(&b)
            .env("BUCKETS_USER", "agent-a")
            .args(["lock", "L_Island.umap"])
            .assert()
            .success();

        let locks = json(&b, &["locks", "--json"]);
        assert_eq!(locks[0]["owner"], "agent-a");

        fs::write(b.join("L_Island.umap"), "m2").unwrap();
        buckets(&b, &["commit", "not mine"])
            .failure()
            .stderr(predicates::str::contains("locked by someone else"));
        buckets(&b, &["unlock", "L_Island.umap"]).failure();

        let mut commit = assert_cmd::Command::cargo_bin("buckets").unwrap();
        commit
            .current_dir(&b)
            .env("BUCKETS_USER", "agent-a")
            .args(["commit", "mine"])
            .assert()
            .success();
        buckets(&b, &["unlock", "--force", "L_Island.umap"]).success();
        assert_eq!(json(&b, &["locks", "--json"]), serde_json::json!([]));
    }

    #[test]
    #[serial]
    fn stash_saves_and_restores_changes() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::write(b.join("keep.txt"), "k1").unwrap();
        fs::write(b.join("gone.txt"), "g1").unwrap();
        buckets(&b, &["commit", "base"]).success();

        fs::write(b.join("keep.txt"), "k2").unwrap();
        fs::remove_file(b.join("gone.txt")).unwrap();
        fs::write(b.join("new.txt"), "n1").unwrap();
        buckets(&b, &["stash", "save", "-m", "wip"]).success();
        assert!(all_committed(&b, &["keep.txt", "gone.txt"]));
        assert!(!b.join("new.txt").exists());

        let list = json(&b, &["stash", "list", "--json"]);
        assert_eq!(list[0]["message"], "wip");

        buckets(&b, &["stash", "pop"]).success();
        assert_eq!(fs::read_to_string(b.join("keep.txt")).unwrap(), "k2");
        assert_eq!(fs::read_to_string(b.join("new.txt")).unwrap(), "n1");
        assert!(!b.join("gone.txt").exists());
        assert_eq!(
            json(&b, &["stash", "list", "--json"]),
            serde_json::json!([])
        );
    }

    #[test]
    #[serial]
    fn push_and_pull_share_objects_through_a_remote_directory() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        let remote = tempfile::tempdir().unwrap();
        let remote_path = remote.path().join("objects");
        buckets(
            &fx.repo_dir,
            &["remote", "set", remote_path.to_str().unwrap()],
        )
        .success();

        fs::write(b.join("a.uasset"), "a1").unwrap();
        fs::write(b.join("b.uasset"), "b1").unwrap();
        buckets(&b, &["commit", "one"]).success();
        let pushed = json(&fx.repo_dir, &["push", "--json"]);
        assert_eq!(pushed["objects_copied"], 2);

        // Simulate a second machine: same database, empty local store and an old checkout.
        fs::remove_dir_all(fx.repo_dir.join(".buckets").join("objects")).unwrap();
        fs::write(b.join("a.uasset"), "a2").unwrap();
        fs::write(b.join("c.uasset"), "c1").unwrap();
        fs::remove_file(b.join("b.uasset")).unwrap();
        buckets(&b, &["commit", "two"]).success();
        buckets(&fx.repo_dir, &["push"]).success();

        // Go back to the first state locally, as if this machine had not seen commit two.
        fs::write(b.join("a.uasset"), "a1").unwrap();
        fs::write(b.join("b.uasset"), "b1").unwrap();
        fs::remove_file(b.join("c.uasset")).unwrap();
        fs::write(b.join("untracked.txt"), "mine").unwrap();
        fs::remove_dir_all(fx.repo_dir.join(".buckets").join("objects")).unwrap();

        let pulled = json(&b, &["pull", "--json"]);
        assert_eq!(pulled[0]["conflicts"], serde_json::json!([]));
        assert_eq!(fs::read_to_string(b.join("a.uasset")).unwrap(), "a2");
        assert_eq!(fs::read_to_string(b.join("c.uasset")).unwrap(), "c1");
        assert!(!b.join("b.uasset").exists(), "deleted upstream");
        assert!(b.join("untracked.txt").exists(), "untracked files are kept");

        // A local edit is never overwritten without --force.
        fs::write(b.join("a.uasset"), "local edit").unwrap();
        buckets(&b, &["pull"]).failure();
        assert_eq!(
            fs::read_to_string(b.join("a.uasset")).unwrap(),
            "local edit"
        );
        buckets(&b, &["pull", "--force"]).success();
        assert_eq!(fs::read_to_string(b.join("a.uasset")).unwrap(), "a2");
    }

    #[test]
    #[serial]
    fn lock_paths_with_dot_dot_still_block_other_owners() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::create_dir_all(b.join("subdir")).unwrap();
        fs::write(b.join("model.blend"), "m1").unwrap();
        buckets(&b, &["commit", "base"]).success();

        let mut lock = assert_cmd::Command::cargo_bin("buckets").unwrap();
        lock.current_dir(&b)
            .env("BUCKETS_USER", "agent-a")
            .args(["lock", "subdir/../model.blend"])
            .assert()
            .success();
        assert_eq!(
            json(&b, &["locks", "--json"])[0]["file_path"],
            "model.blend"
        );

        fs::write(b.join("model.blend"), "m2").unwrap();
        buckets(&b, &["commit", "not mine"])
            .failure()
            .stderr(predicates::str::contains("locked by someone else"));
        buckets(&b, &["lock", "../outside.txt"]).failure();
    }

    #[test]
    #[serial]
    fn revert_to_an_empty_commit_removes_files() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::write(b.join("a.txt"), "a1").unwrap();
        buckets(&b, &["commit", "one"]).success();
        fs::remove_file(b.join("a.txt")).unwrap();
        let empty = json(&b, &["commit", "empty", "--json"])["commit_id"]
            .as_str()
            .unwrap()
            .to_string();
        fs::write(b.join("a.txt"), "a2").unwrap();
        buckets(&b, &["commit", "back"]).success();

        buckets(&b, &["revert", "--commit", &empty]).success();
        assert!(!b.join("a.txt").exists());
    }

    #[test]
    #[serial]
    fn pull_handles_file_to_directory_changes() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        let remote = tempfile::tempdir().unwrap();
        buckets(
            &fx.repo_dir,
            &["remote", "set", remote.path().to_str().unwrap()],
        )
        .success();

        // Commit one: `asset` is a file.
        fs::write(b.join("asset"), "flat").unwrap();
        buckets(&b, &["commit", "flat"]).success();
        buckets(&fx.repo_dir, &["push"]).success();

        // Commit two: `asset` becomes a directory.
        fs::remove_file(b.join("asset")).unwrap();
        fs::create_dir_all(b.join("asset")).unwrap();
        fs::write(b.join("asset").join("model.blend"), "nested").unwrap();
        buckets(&b, &["commit", "nested"]).success();
        buckets(&fx.repo_dir, &["push"]).success();

        // Old checkout with the file: pull turns it into the directory.
        fs::remove_dir_all(b.join("asset")).unwrap();
        fs::write(b.join("asset"), "flat").unwrap();
        let pulled = json(&b, &["pull", "--json"]);
        assert_eq!(pulled[0]["conflicts"], serde_json::json!([]));
        assert_eq!(
            fs::read_to_string(b.join("asset").join("model.blend")).unwrap(),
            "nested"
        );

        // And back: directory becomes a file again.
        fs::remove_dir_all(b.join("asset")).unwrap();
        fs::write(b.join("asset"), "flat again").unwrap();
        buckets(&b, &["commit", "flat again"]).success();
        buckets(&fx.repo_dir, &["push"]).success();
        fs::remove_file(b.join("asset")).unwrap();
        fs::create_dir_all(b.join("asset")).unwrap();
        fs::write(b.join("asset").join("model.blend"), "nested").unwrap();
        let pulled = json(&b, &["pull", "--json"]);
        assert_eq!(pulled[0]["conflicts"], serde_json::json!([]));
        assert_eq!(fs::read_to_string(b.join("asset")).unwrap(), "flat again");
    }

    #[test]
    #[serial]
    fn revert_handles_file_to_directory_changes() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::create_dir_all(b.join("asset")).unwrap();
        fs::write(b.join("asset").join("model.blend"), "nested").unwrap();
        let nested = json(&b, &["commit", "nested", "--json"])["commit_id"]
            .as_str()
            .unwrap()
            .to_string();
        fs::remove_dir_all(b.join("asset")).unwrap();
        fs::write(b.join("asset"), "flat").unwrap();
        let flat = json(&b, &["commit", "flat", "--json"])["commit_id"]
            .as_str()
            .unwrap()
            .to_string();

        buckets(&b, &["revert", "--commit", &nested]).success();
        assert_eq!(
            fs::read_to_string(b.join("asset").join("model.blend")).unwrap(),
            "nested"
        );
        buckets(&b, &["revert", "--commit", &flat]).success();
        assert_eq!(fs::read_to_string(b.join("asset")).unwrap(), "flat");
    }

    #[test]
    #[serial]
    fn stash_handles_file_to_directory_changes() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::write(b.join("asset"), "flat").unwrap();
        buckets(&b, &["commit", "flat"]).success();

        // File replaced by a directory, stashed and popped.
        fs::remove_file(b.join("asset")).unwrap();
        fs::create_dir_all(b.join("asset")).unwrap();
        fs::write(b.join("asset").join("model.blend"), "nested").unwrap();
        buckets(&b, &["stash", "save"]).success();
        assert_eq!(fs::read_to_string(b.join("asset")).unwrap(), "flat");
        buckets(&b, &["stash", "pop"]).success();
        assert_eq!(
            fs::read_to_string(b.join("asset").join("model.blend")).unwrap(),
            "nested"
        );

        // Commit the directory, then replace it by a file and stash that.
        buckets(&b, &["commit", "nested"]).success();
        fs::remove_dir_all(b.join("asset")).unwrap();
        fs::write(b.join("asset"), "flat again").unwrap();
        buckets(&b, &["stash", "save"]).success();
        assert_eq!(
            fs::read_to_string(b.join("asset").join("model.blend")).unwrap(),
            "nested"
        );
        buckets(&b, &["stash", "pop"]).success();
        assert_eq!(fs::read_to_string(b.join("asset")).unwrap(), "flat again");
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn pull_never_writes_through_a_symlinked_directory() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::create_dir_all(b.join("assets")).unwrap();
        fs::write(b.join("assets").join("model.blend"), "v1").unwrap();
        buckets(&b, &["commit", "v1"]).success();

        // Replace the directory with a symlink to a directory outside the bucket.
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("model.blend"), "not yours").unwrap();
        fs::remove_dir_all(b.join("assets")).unwrap();
        std::os::unix::fs::symlink(outside.path(), b.join("assets")).unwrap();

        buckets(&b, &["pull", "--force"]).failure();
        assert_eq!(
            fs::read_to_string(outside.path().join("model.blend")).unwrap(),
            "not yours"
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn stash_and_revert_never_write_through_a_symlinked_directory() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::create_dir_all(b.join("assets")).unwrap();
        fs::write(b.join("assets").join("model.blend"), "v1").unwrap();
        let first = json(&b, &["commit", "v1", "--json"])["commit_id"]
            .as_str()
            .unwrap()
            .to_string();

        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("model.blend"), "not yours").unwrap();
        fs::remove_dir_all(b.join("assets")).unwrap();
        std::os::unix::fs::symlink(outside.path(), b.join("assets")).unwrap();

        buckets(&b, &["stash", "save"]).failure();
        assert_eq!(
            json(&b, &["stash", "list", "--json"]),
            serde_json::json!([])
        );
        buckets(&b, &["rollback"]).failure();
        assert_eq!(
            fs::read_to_string(outside.path().join("model.blend")).unwrap(),
            "not yours"
        );

        // A stash that deletes a file inside the folder never deletes through the link.
        fs::remove_file(b.join("assets")).unwrap();
        fs::create_dir_all(b.join("assets")).unwrap();
        buckets(&b, &["stash", "save"]).success(); // records deletion of assets/model.blend
        fs::remove_dir_all(b.join("assets")).unwrap();
        std::os::unix::fs::symlink(outside.path(), b.join("assets")).unwrap();
        buckets(&b, &["stash", "pop", "--force"]).failure();
        assert_eq!(
            fs::read_to_string(outside.path().join("model.blend")).unwrap(),
            "not yours"
        );
        buckets(&b, &["stash", "drop"]).success();

        // A full revert removes the link itself and restores a real folder.
        buckets(&b, &["revert", "--commit", &first]).success();
        assert_eq!(
            fs::read_to_string(outside.path().join("model.blend")).unwrap(),
            "not yours"
        );
        assert!(!fs::symlink_metadata(b.join("assets"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            fs::read_to_string(b.join("assets").join("model.blend")).unwrap(),
            "v1"
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial]
    fn names_that_escape_the_bucket_are_never_committed() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::write(b.join("..\\outside.txt"), "x").unwrap();
        buckets(&b, &["commit", "escape"])
            .failure()
            .stderr(predicates::str::contains("outside the bucket"));
        // A literal backslash would turn into a folder on other systems.
        fs::remove_file(b.join("..\\outside.txt")).unwrap();
        fs::write(b.join("assets\\model.blend"), "x").unwrap();
        buckets(&b, &["commit", "backslash"])
            .failure()
            .stderr(predicates::str::contains("change meaning"));
    }

    #[test]
    #[serial]
    fn stash_refuses_names_that_escape_the_bucket() {
        let Some(fx) = fixture_or_skip() else { return };
        let b = fx.bucket_dir.clone();
        fs::write(b.join("keep.txt"), "v1").unwrap();
        buckets(&b, &["commit", "first"]).success();
        fs::write(b.join("..\\outside.txt"), "x").unwrap();
        buckets(&b, &["stash"])
            .failure()
            .stderr(predicates::str::contains("leave the bucket"));
        // Nothing was recorded or removed.
        assert!(b.join("..\\outside.txt").exists());
        assert!(!b.join(".b").join("stash.json").exists());
    }
}
