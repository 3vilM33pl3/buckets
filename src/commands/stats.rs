use crate::args::StatsCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::Bucket;
use crate::errors::BucketError;
use crate::postgres_db::get_database;
use crate::utils::checks;
use crate::utils::runtime::RuntimeManager;
use crate::CURRENT_DIR;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tokio_postgres::types::ToSql;

#[derive(Serialize, Deserialize)]
pub struct StatsOutput {
    bucket_count: usize,
    total_commits: usize,
    total_files: usize,
    buckets: Vec<BucketStats>,
}

#[derive(Serialize, Deserialize)]
pub struct BucketStats {
    name: String,
    id: String,
    commit_count: usize,
    file_count: usize,
}

/// Stats command to show repository statistics
pub struct Stats {
    #[allow(dead_code)]
    args: StatsCommand,
}

impl BucketCommand for Stats {
    type Args = StatsCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let current_dir = CURRENT_DIR.with(|dir| dir.clone());
        debug!("Generating stats for directory: {:?}", current_dir);

        if !checks::is_valid_bucket_repo(&current_dir) {
            debug!("Not in a valid bucket repository");
            return Err(BucketError::NotInRepo);
        }

        info!("Gathering repository statistics");

        let runtime_handle = RuntimeManager::handle()?;

        let buckets = runtime_handle.block_on(self.query_buckets_async())?;
        debug!("Found {} buckets", buckets.len());

        let total_commits = runtime_handle.block_on(self.count_total_commits_async())?;
        debug!("Found {} total commits", total_commits);

        let total_files = runtime_handle.block_on(self.count_total_files_async())?;
        debug!("Found {} total files", total_files);

        let mut bucket_stats = Vec::new();
        for bucket in &buckets {
            let commit_count = runtime_handle
                .block_on(self.count_bucket_commits_async(&bucket.id))
                .unwrap_or(0);
            let file_count = runtime_handle
                .block_on(self.count_bucket_files_async(&bucket.id))
                .unwrap_or(0);
            bucket_stats.push(BucketStats {
                name: bucket.name.clone(),
                id: bucket.id.to_string(),
                commit_count,
                file_count,
            });
        }

        if self.args.shared.json {
            let output = StatsOutput {
                bucket_count: buckets.len(),
                total_commits,
                total_files,
                buckets: bucket_stats,
            };
            match serde_json::to_string_pretty(&output) {
                Ok(json) => println!("{}", json),
                Err(e) => eprintln!("Error serializing to JSON: {}", e),
            }
        } else {
            println!("Repository Statistics:");
            println!("  Buckets: {}", buckets.len());
            println!("  Total Commits: {}", total_commits);
            println!("  Total Files: {}", total_files);
            println!();

            if !bucket_stats.is_empty() {
                println!("Bucket Statistics:");
                for stats in &bucket_stats {
                    println!("  {} ({}):", stats.name, stats.id);
                    println!("    Commits: {}", stats.commit_count);
                    println!("    Files: {}", stats.file_count);
                    println!();
                }
            }
        }

        Ok(())
    }
}

impl Stats {
    async fn query_buckets_async(&self) -> Result<Vec<Bucket>, BucketError> {
        let db = get_database().await?;

        let rows = db.query("SELECT id, name, path FROM buckets", &[]).await?;

        let mut buckets = Vec::new();
        for row in &rows {
            let uuid: uuid::Uuid = row.get(0);
            let name: String = row.get(1);
            let path_str: String = row.get(2);

            buckets.push(Bucket {
                id: uuid,
                name,
                relative_bucket_path: std::path::PathBuf::from(path_str),
            });
        }

        Ok(buckets)
    }

    async fn count_total_commits_async(&self) -> Result<usize, BucketError> {
        let db = get_database().await?;

        let rows = db.query("SELECT COUNT(*) FROM commits", &[]).await?;
        let count: i64 = rows[0].get(0);
        Ok(count as usize)
    }

    async fn count_total_files_async(&self) -> Result<usize, BucketError> {
        let db = get_database().await?;

        let rows = db
            .query("SELECT COUNT(DISTINCT file_path) FROM files", &[])
            .await?;
        let count: i64 = rows[0].get(0);
        Ok(count as usize)
    }

    async fn count_bucket_commits_async(
        &self,
        bucket_id: &uuid::Uuid,
    ) -> Result<usize, BucketError> {
        let db = get_database().await?;

        let params: Vec<&(dyn ToSql + Sync)> = vec![bucket_id];

        let rows = db
            .query("SELECT COUNT(*) FROM commits WHERE bucket_id = $1", &params)
            .await?;
        let count: i64 = rows[0].get(0);
        Ok(count as usize)
    }

    async fn count_bucket_files_async(&self, bucket_id: &uuid::Uuid) -> Result<usize, BucketError> {
        let db = get_database().await?;

        let params: Vec<&(dyn ToSql + Sync)> = vec![bucket_id];

        let rows = db.query(
            "SELECT COUNT(DISTINCT f.file_path) FROM files f JOIN commits c ON f.commit_id = c.id WHERE c.bucket_id = $1", 
            &params
        ).await?;
        let count: i64 = rows[0].get(0);
        Ok(count as usize)
    }
}
