mod common;

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn test_config_global_set_get_list_unset() {
        let temp_home = tempdir().expect("failed to create temp home");

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .arg("config")
            .arg("set")
            .arg("network.ntp_server")
            .arg("time.google.com")
            .arg("--global")
            .assert()
            .success();

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .arg("config")
            .arg("get")
            .arg("network.ntp_server")
            .arg("--global")
            .assert()
            .success()
            .stdout(predicate::str::contains("time.google.com"));

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .arg("config")
            .arg("list")
            .arg("--global")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("[network]")
                    .and(predicate::str::contains("ntp_server = \"time.google.com\"")),
            );

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .arg("config")
            .arg("unset")
            .arg("network.ntp_server")
            .arg("--global")
            .assert()
            .success();

        let config_path = temp_home.path().join(".buckets_config.toml");
        let content = fs::read_to_string(config_path).expect("missing global config");
        assert!(!content.contains("ntp_server"));
    }

    #[test]
    #[serial]
    fn test_config_effective_prefers_local() {
        let temp_home = tempdir().expect("failed to create temp home");
        let repo_dir = tempdir().expect("failed to create repo dir");
        fs::create_dir_all(repo_dir.path().join(".buckets")).expect("failed to create .buckets");

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .current_dir(repo_dir.path())
            .arg("config")
            .arg("set")
            .arg("network.ntp_server")
            .arg("global.ntp")
            .arg("--global")
            .assert()
            .success();

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .current_dir(repo_dir.path())
            .arg("config")
            .arg("set")
            .arg("network.ntp_server")
            .arg("local.ntp")
            .arg("--local")
            .assert()
            .success();

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .current_dir(repo_dir.path())
            .arg("config")
            .arg("get")
            .arg("network.ntp_server")
            .assert()
            .success()
            .stdout(predicate::str::contains("local.ntp"));

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .current_dir(repo_dir.path())
            .arg("config")
            .arg("list")
            .arg("--effective")
            .assert()
            .success()
            .stdout(predicate::str::contains("local.ntp"));
    }
}
