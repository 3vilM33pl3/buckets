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
    fn test_cli_setup_json_outputs_default_config() {
        let temp_home = tempdir().expect("failed to create temp home");

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .arg("setup")
            .arg("--json")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("\"ntp_server\"")
                    .and(predicate::str::contains("pool.ntp.org"))
                    .and(predicate::str::contains("\"postgresql_connection\"")),
            );

        let config_path = temp_home.path().join(".buckets_config.toml");
        assert!(
            !config_path.exists(),
            "json mode should not create config file"
        );
    }

    #[test]
    #[serial]
    fn test_cli_setup_json_outputs_existing_config() {
        let temp_home = tempdir().expect("failed to create temp home");
        let config_path = temp_home.path().join(".buckets_config.toml");
        let config_content = "ntp_server = \"time.google.com\"\npostgresql_connection = \"postgresql://user:pass@localhost:5432/buckets\"\n";
        fs::write(&config_path, config_content).expect("failed to write config file");

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .arg("setup")
            .arg("--json")
            .assert()
            .success()
            .stdout(
                predicate::str::contains("time.google.com")
                    .and(
                        predicate::str::contains("postgresql://user:***@localhost:5432/buckets")
                            .not(),
                    )
                    .and(predicate::str::contains(
                        "postgresql://user:pass@localhost:5432/buckets",
                    )),
            );
    }

    #[test]
    #[serial]
    fn test_cli_setup_json_rejects_test_connection() {
        let temp_home = tempdir().expect("failed to create temp home");

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.env("HOME", temp_home.path())
            .arg("setup")
            .arg("--json")
            .arg("--test-connection")
            .assert()
            .failure()
            .stderr(predicate::str::contains("--json cannot be combined"));
    }
}
