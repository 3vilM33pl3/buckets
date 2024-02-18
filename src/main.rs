mod commands;
mod data;
mod utils;

use clap::{arg, Command};
use std::io;

fn cli() -> Command {
    Command::new("bucket")
        .version("1.0")
        .author("3vilM33pl3 <olivier@robotmotel.com>")
        .about("")
        .subcommand(Command::new("version").about("Displays the version of the bucket tool"))
        .subcommand(
            Command::new("init")
                .about("Initialises bucket repository")
                .about("Initialises bucket repository")
                .arg(arg!(<NAME> "Name of the repository"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("create")
                .about("Creates a new bucket")
                .about("Creates a new bucket")
                .arg(arg!(<NAME> "Name of the bucket"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("commit").about("Commits a bucket"))
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        None => {}
        Some(("version", _)) => commands::version::execute(&mut io::stdout()).unwrap(),
        Some(("init", sub_matches)) => {
            let arg = sub_matches.get_one::<String>("NAME").unwrap();

            if let Err(e) = commands::init::execute(&arg.to_string()) {
                println!("Can not create repository: {}", e);
            } else {
                println!("Initialised bucket repository");
            }
        }
        Some(("create", sub_matches)) => {
            let arg = sub_matches.get_one::<String>("NAME").unwrap();

            if let Err(e) = commands::create::execute(&arg.to_string()) {
                eprintln!("Can not create bucket: {}", e);
            } else {
                println!("Created bucket");
            }
        }
        Some(("commit", _)) => {
            if let Err(e) = commands::commit::execute() {
                println!("Can not commit bucket: {}", e);
            } else {
                println!("Committed bucket");
            }
        }
        _ => commands::version::execute(&mut io::stdout()).unwrap(),
    }
}

#[cfg(test)]
use tempfile::tempdir;
#[cfg(test)]
mod tests {

    use super::*;
    use coverage_helper::test;
    use predicates::prelude::predicate;

    #[test]
    #[ignore]
    fn test_cli_version() {
        let mut cmd = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd.assert().success();
        cmd.arg("version").assert().stdout("bucket version 0.1.0\n");
    }

    #[test]
    #[ignore]
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

    #[test]
    #[ignore]
    fn test_create_fail() {
        let temp_dir = tempdir().unwrap();
        let mut cmd_create = assert_cmd::Command::cargo_bin("buckets").unwrap();
        cmd_create.current_dir(temp_dir.path());
        cmd_create
            .arg("create")
            .arg("test_bucket")
            .assert()
            .success()
            .stderr(predicate::str::contains(
                "Can not create bucket: Not in a bucket repository",
            ));
    }

    #[test]
    #[ignore]
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

    #[test]
    #[ignore]
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
