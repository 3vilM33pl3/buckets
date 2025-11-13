#[cfg(test)]
pub mod tests {
    use assert_cmd::Command;
    use once_cell::sync::Lazy;
    use std::path::{Path, PathBuf};
    use std::process::{Command as ProcessCommand, Stdio};
    use std::thread;
    use std::time::Duration;
    use std::{env, fs};
    use tempfile::tempdir;
    use testcontainers::{clients::Cli, core::WaitFor, Container, GenericImage};
    use tokio_postgres::NoTls;
    use uuid::Uuid;

    static DOCKER: Lazy<Cli> = Lazy::new(|| Cli::default());

    const POSTGRES_IMAGE: &str = "postgres";
    const POSTGRES_TAG: &str = "16-alpine";

    #[allow(dead_code)]
    pub fn get_test_dir() -> PathBuf {
        match env::var("TEST_DIR") {
            Ok(val) => PathBuf::from(val),
            Err(_) => tempdir().expect("error creating temp dir").keep(),
        }
    }

    #[allow(dead_code)]
    fn docker_tests_disabled() -> bool {
        const SKIP_VARS: [&str; 3] = [
            "BUCKETS_SKIP_DOCKER_TESTS",
            "BUCKETS_SKIP_DB_TESTS",
            "NO_NETWORK",
        ];

        SKIP_VARS.iter().any(|var| match env::var(var) {
            Ok(value) => {
                let normalized = value.to_ascii_lowercase();
                value.is_empty() || normalized == "1" || normalized == "true" || normalized == "yes"
            }
            Err(_) => false,
        })
    }

    fn docker_command_available() -> bool {
        ProcessCommand::new("docker")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    pub struct TestDatabase {
        connection_string: String,
        #[allow(dead_code)]
        container: Container<'static, GenericImage>,
        previous_database_url: Option<String>,
    }

    #[allow(dead_code)]
    impl TestDatabase {
        pub fn new() -> Result<Self, String> {
            if docker_tests_disabled() {
                return Err(
                    "Skipping Docker-dependent test (set BUCKETS_SKIP_DOCKER_TESTS=0 to enable)."
                        .to_string(),
                );
            }

            if !docker_command_available() {
                return Err(
                    "Skipping Docker-dependent test because the docker CLI was not found"
                        .to_string(),
                );
            }

            let db_name = format!("buckets_test_{}", Uuid::new_v4().simple());

            let image = GenericImage::new(POSTGRES_IMAGE, POSTGRES_TAG)
                .with_env_var("POSTGRES_USER", "buckets")
                .with_env_var("POSTGRES_PASSWORD", "password")
                .with_env_var("POSTGRES_DB", &db_name)
                .with_exposed_port(5432)
                .with_wait_for(WaitFor::message_on_stdout(
                    "database system is ready to accept connections",
                ));

            let container = DOCKER.run(image);
            let host_port = container.get_host_port_ipv4(5432);
            let connection_string =
                format!("postgresql://buckets:password@127.0.0.1:{host_port}/{db_name}");

            let previous_database_url = env::var("DATABASE_URL").ok();
            env::set_var("DATABASE_URL", &connection_string);

            if wait_for_postgres_ready().is_none() {
                if let Some(prev) = &previous_database_url {
                    env::set_var("DATABASE_URL", prev);
                } else {
                    env::remove_var("DATABASE_URL");
                }
                return Err("PostgreSQL container did not become ready in time".to_string());
            }

            Ok(Self {
                connection_string,
                container,
                previous_database_url,
            })
        }

        pub fn apply_database_url(&self) {
            // DATABASE_URL already set in new(); nothing additional required but keep helper
        }

        pub fn connection_string(&self) -> &str {
            &self.connection_string
        }
    }

    impl Drop for TestDatabase {
        fn drop(&mut self) {
            if let Some(prev) = &self.previous_database_url {
                env::set_var("DATABASE_URL", prev);
            } else {
                env::remove_var("DATABASE_URL");
            }
        }
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    pub struct RepoFixture {
        pub repo_dir: PathBuf,
        pub bucket_dir: PathBuf,
        #[allow(dead_code)]
        temp_dir: PathBuf,
        #[allow(dead_code)]
        db: TestDatabase,
    }

    #[allow(dead_code)]
    impl RepoFixture {
        pub fn new() -> Result<Self, String> {
            let db = TestDatabase::new()?;
            db.apply_database_url();

            let temp_dir = get_test_dir();
            let repo_name = format!("repo_{}", Uuid::new_v4().simple());
            let bucket_name = format!("bucket_{}", Uuid::new_v4().simple());

            run_buckets_command(&temp_dir, ["init", &repo_name]);
            let repo_dir = temp_dir.join(&repo_name);
            let buckets_dir = repo_dir.join(".buckets");

            if !buckets_dir.exists() {
                return Err(format!(
                    "Init command did not create .buckets at {}",
                    buckets_dir.display()
                ));
            }

            let db_type_file = buckets_dir.join("database_type");
            if !db_type_file.exists() {
                std::fs::write(&db_type_file, "PostgreSQL").map_err(|e| {
                    format!(
                        "Failed to write database_type file at {}: {}",
                        db_type_file.display(),
                        e
                    )
                })?;
            }

            let bucket_dir = provision_bucket(&repo_dir, &bucket_name, db.connection_string())?;

            Ok(Self {
                repo_dir,
                bucket_dir,
                temp_dir,
                db,
            })
        }
    }

    #[allow(dead_code)]
    fn run_buckets_command<'a>(dir: &Path, args: impl IntoIterator<Item = &'a str>) {
        let mut cmd = Command::cargo_bin("buckets").expect("failed to run command");
        cmd.current_dir(dir);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.assert().success();
    }

    fn wait_for_postgres_ready() -> Option<()> {
        let connection_string = env::var("DATABASE_URL").ok()?;
        let runtime = tokio::runtime::Runtime::new().ok()?;

        for _ in 0..10 {
            let attempt = runtime.block_on(async {
                match tokio_postgres::connect(&connection_string, NoTls).await {
                    Ok((client, connection)) => {
                        tokio::spawn(async move {
                            let _ = connection.await;
                        });
                        client.simple_query("SELECT 1").await.map(|_| ())
                    }
                    Err(e) => Err(e),
                }
            });

            if attempt.is_ok() {
                return Some(());
            }

            thread::sleep(Duration::from_millis(200));
        }

        None
    }

    fn provision_bucket(
        repo_dir: &Path,
        bucket_name: &str,
        connection_string: &str,
    ) -> Result<PathBuf, String> {
        let bucket_dir = repo_dir.join(bucket_name);
        let storage_dir = bucket_dir.join(".b").join("storage");
        fs::create_dir_all(&storage_dir)
            .map_err(|e| format!("Failed to create bucket storage: {}", e))?;

        let bucket_id = Uuid::new_v4();
        let info_content = format!(
            "id = \"{}\"\nname = \"{}\"\nrelative_bucket_path = \"{}\"\n",
            bucket_id, bucket_name, bucket_name
        );
        let info_path = bucket_dir.join(".b").join("info");
        fs::write(&info_path, info_content)
            .map_err(|e| format!("Failed to write bucket info: {}", e))?;

        let runtime =
            tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {}", e))?;
        let escaped_name = bucket_name.replace('\'', "''");
        let insert_sql = format!(
            "INSERT INTO buckets (id, name, path, created_at) VALUES ('{}'::uuid, '{}'::text, '{}'::text, NOW())",
            bucket_id, escaped_name, escaped_name
        );
        runtime
            .block_on(async {
                let (client, connection) = tokio_postgres::connect(connection_string, NoTls)
                    .await
                    .map_err(|e| e.to_string())?;
                tokio::spawn(async move {
                    let _ = connection.await;
                });
                client
                    .batch_execute(&insert_sql)
                    .await
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| format!("Failed to insert bucket row: {}", e))?;

        Ok(bucket_dir)
    }
}
