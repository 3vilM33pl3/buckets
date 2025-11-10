#![cfg(test)]

pub mod docker {
    use once_cell::sync::Lazy;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;
    use std::{env, fs};
    use testcontainers::{clients::Cli, core::WaitFor, Container, GenericImage};
    use tokio_postgres::NoTls;
    use uuid::Uuid;

    static DOCKER: Lazy<Cli> = Lazy::new(|| Cli::default());
    const POSTGRES_IMAGE: &str = "postgres";
    const POSTGRES_TAG: &str = "16-alpine";

    #[derive(Debug)]
    pub struct TestDatabase {
        connection_string: String,
        _container: Container<'static, GenericImage>,
        previous_database_url: Option<String>,
    }

    impl TestDatabase {
        pub fn new() -> Option<Self> {
            if docker_tests_disabled() {
                return None;
            }

            if !docker_command_available() {
                return None;
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
                return None;
            }

            Some(Self {
                connection_string,
                _container: container,
                previous_database_url,
            })
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
        Command::new("docker")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn wait_for_postgres_ready() -> Option<()> {
        let connection_string =
            env::var("DATABASE_URL").expect("DATABASE_URL must be set before waiting");
        let runtime = tokio::runtime::Runtime::new().ok()?;

        for _ in 0..10 {
            let attempt = runtime.block_on(async {
                match tokio_postgres::connect(&connection_string, NoTls).await {
                    Ok((client, connection)) => {
                        tokio::spawn(async move {
                            let _ = connection.await;
                        });
                        if let Err(e) = client.simple_query("SELECT 1").await {
                            Err(e)
                        } else {
                            Ok(())
                        }
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

    pub fn create_bucket_for_tests(
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

        let escaped_name = bucket_name.replace('\'', "''");
        let insert_sql = format!(
            "INSERT INTO buckets (id, name, path, created_at) VALUES ('{}'::uuid, '{}'::text, '{}'::text, NOW())",
            bucket_id, escaped_name, escaped_name
        );

        let runtime =
            tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {}", e))?;
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

    pub fn ensure_database_initialized(connection_string: &str) {
        let config = crate::postgres_db::DatabaseConfig::from_url(connection_string)
            .expect("Invalid connection string for test database");
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime for DB init");
        runtime
            .block_on(async move {
                crate::postgres_db::reset_database_for_tests().await;
                crate::postgres_db::init_database(config).await
            })
            .expect("Failed to initialize test database");
    }
}
