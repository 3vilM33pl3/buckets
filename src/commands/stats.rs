use crate::args::StatsCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::Bucket;
use crate::errors::BucketError;
use crate::utils::checks;
use crate::utils::utils::with_db_connection;
use crate::CURRENT_DIR;
use log::{debug, info};
use serde::{Deserialize, Serialize};

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
        let buckets = self.query_buckets()?;
        debug!("Found {} buckets", buckets.len());
        
        let total_commits = self.count_total_commits()?;
        debug!("Found {} total commits", total_commits);
        
        let total_files = self.count_total_files()?;
        debug!("Found {} total files", total_files);
        
        let bucket_stats: Vec<BucketStats> = buckets.iter().map(|bucket| {
            let commit_count = self.count_bucket_commits(&bucket.id).unwrap_or(0);
            let file_count = self.count_bucket_files(&bucket.id).unwrap_or(0);
            BucketStats {
                name: bucket.name.clone(),
                id: bucket.id.to_string(),
                commit_count,
                file_count,
            }
        }).collect();
        
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
    fn query_buckets(&self) -> Result<Vec<Bucket>, BucketError> {
        with_db_connection(|connection| {
            let mut stmt = connection.prepare("SELECT id, name, path FROM buckets")?;
            let bucket_iter = stmt
                .query_map([], |row| {
                    let uuid_str: String = row.get(0)?;
                    let path_str: String = row.get(2)?;
                    let uuid = uuid::Uuid::parse_str(&uuid_str)
                        .map_err(|e| BucketError::InvalidData(e.to_string()))?;
                    Ok(Bucket {
                        id: uuid,
                        name: row.get(1)?,
                        relative_bucket_path: std::path::PathBuf::from(path_str),
                    })
                })
                .map_err(BucketError::from)?;
            let mut buckets = Vec::new();
            for bucket in bucket_iter {
                buckets.push(bucket.map_err(BucketError::from)?);
            }

            Ok(buckets)
        })
    }
    
    fn count_total_commits(&self) -> Result<usize, BucketError> {
        with_db_connection(|connection| {
            let mut stmt = connection.prepare("SELECT COUNT(*) FROM commits")?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            Ok(count as usize)
        })
    }
    
    fn count_total_files(&self) -> Result<usize, BucketError> {
        with_db_connection(|connection| {
            let mut stmt = connection.prepare("SELECT COUNT(DISTINCT file_path) FROM files")?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            Ok(count as usize)
        })
    }
    
    fn count_bucket_commits(&self, bucket_id: &uuid::Uuid) -> Result<usize, BucketError> {
        with_db_connection(|connection| {
            let mut stmt = connection.prepare("SELECT COUNT(*) FROM commits WHERE bucket_id = ?")?;
            let count: i64 = stmt.query_row([bucket_id.to_string()], |row| row.get(0))?;
            Ok(count as usize)
        })
    }
    
    fn count_bucket_files(&self, bucket_id: &uuid::Uuid) -> Result<usize, BucketError> {
        with_db_connection(|connection| {
            let mut stmt = connection.prepare("SELECT COUNT(DISTINCT f.file_path) FROM files f JOIN commits c ON f.commit_id = c.id WHERE c.bucket_id = ?")?;
            let count: i64 = stmt.query_row([bucket_id.to_string()], |row| row.get(0))?;
            Ok(count as usize)
        })
    }
}
