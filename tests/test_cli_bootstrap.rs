mod common;

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    use serial_test::serial;
    use std::fs;

    use std::panic;
    use uuid::Uuid;

    use crate::common::tests::{get_test_dir, RepoFixture, TestDatabase};

    #[test]
    #[serial]
    fn bootstrap_uses_repository_configuration() {
        let Some(fixture) = repo_fixture_or_skip() else {
            return;
        };

        let previous_database_url = std::env::var("DATABASE_URL").ok();
        std::env::remove_var("DATABASE_URL");

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(fixture.repo_dir.as_path())
            .arg("status")
            .assert()
            .success()
            .stdout(predicate::str::contains("Repository config"));

        // Restore DATABASE_URL to avoid affecting other tests
        restore_database_url(previous_database_url);
    }

    #[test]
    #[serial]
    fn bootstrap_errors_when_connection_missing() {
        let temp_dir = get_test_dir();
        // Mock HOME to avoid picking up global config
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp_dir);

        let repo_dir = temp_dir.join(format!("bootstrap_missing_{}", Uuid::new_v4().simple()));
        let buckets_dir = repo_dir.join(".buckets");
        fs::create_dir_all(&buckets_dir).expect("failed to create .buckets");
        fs::write(buckets_dir.join("database_type"), "PostgreSQL")
            .expect("failed to write database_type");

        // Do NOT write db_config.toml.
        // Fallback to global config (which is empty due to mocked HOME).
        // Should error 'No PostgreSQL connection found'.

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(&repo_dir)
            .env("HOME", &temp_dir) // Ensure subprocess sees the mocked HOME
            .env_remove("DATABASE_URL")
            .arg("status")
            .assert()
            .failure()
            .stderr(predicate::str::contains("No PostgreSQL connection"));

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    #[serial]
    fn bootstrap_reports_invalid_credentials() {
        let temp_dir = get_test_dir();
        // Mock HOME
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp_dir);

        let repo_dir = temp_dir.join(format!("bootstrap_invalid_{}", Uuid::new_v4().simple()));
        let buckets_dir = repo_dir.join(".buckets");
        fs::create_dir_all(&buckets_dir).expect("failed to create .buckets");
        fs::write(buckets_dir.join("database_type"), "PostgreSQL")
            .expect("failed to write database_type");

        let Some(database) = test_database_or_skip() else {
            return;
        };

        let connection = database.connection_string();
        // Parse connection string: postgresql://buckets:password@127.0.0.1:<port>/<db_name>
        let after_at = connection
            .split('@')
            .nth(1)
            .expect("invalid connection string format");
        let after_colon = after_at
            .split(':')
            .nth(1)
            .expect("invalid connection string format");
        let mut parts = after_colon.split('/');
        let port: u16 = parts
            .next()
            .expect("missing port")
            .parse()
            .expect("invalid port");
        let db_name = parts.next().expect("missing db name");

        // Write db_config.toml with wrong password
        fs::write(
            buckets_dir.join("db_config.toml"),
            format!(
                "type = \"external\"\nhost = \"127.0.0.1\"\nport = {}\ndatabase = \"{}\"\nusername = \"buckets\"\npassword = \"wrong-password\"\n",
                port, db_name
            ),
        )
        .expect("failed to write config");

        let previous_database_url = std::env::var("DATABASE_URL").ok();
        std::env::remove_var("DATABASE_URL");

        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(&repo_dir)
            .env("HOME", &temp_dir)
            .arg("status")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Failed to initialize PostgreSQL"));

        drop(database);
        restore_database_url(previous_database_url);

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    fn repo_fixture_or_skip() -> Option<RepoFixture> {
        match panic::catch_unwind(|| RepoFixture::new()) {
            Ok(Ok(fixture)) => Some(fixture),
            Ok(Err(message)) => {
                eprintln!("Skipping bootstrap success test: {message}");
                None
            }
            Err(_) => {
                eprintln!("Skipping bootstrap success test: Docker is unavailable");
                None
            }
        }
    }

    fn test_database_or_skip() -> Option<TestDatabase> {
        match panic::catch_unwind(|| TestDatabase::new()) {
            Ok(Ok(db)) => Some(db),
            Ok(Err(message)) => {
                eprintln!("Skipping bootstrap invalid credential test: {message}");
                None
            }
            Err(_) => {
                eprintln!("Skipping bootstrap invalid credential test: Docker is unavailable");
                None
            }
        }
    }

    fn restore_database_url(previous: Option<String>) {
        if let Some(value) = previous {
            std::env::set_var("DATABASE_URL", value);
        } else {
            std::env::remove_var("DATABASE_URL");
        }
    }
}
