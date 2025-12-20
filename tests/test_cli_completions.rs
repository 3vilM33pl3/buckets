#[cfg(test)]
mod tests {
    use predicates::str::contains;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_cli_completions_zsh() {
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").expect("failed to run command");
        cmd.arg("completions")
            .arg("zsh")
            .assert()
            .success()
            .stdout(contains("#compdef buckets"));
    }
}

