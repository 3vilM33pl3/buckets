use crate::args::CheckCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::{Bucket, BucketTrait};
use crate::data::pebble::Pebble;
use crate::errors::BucketError;
use crate::postgres_db::get_database;
use crate::utils::runtime::RuntimeManager;
use crate::CURRENT_DIR;
use dialoguer::console::style;

pub struct Check {
    args: CheckCommand,
}

impl BucketCommand for Check {
    type Args = CheckCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let bucket_name = &self.args.bucket;

        RuntimeManager::block_on(async {
            let db = get_database().await?;

            // 1. Resolve Bucket ID
            let bucket_id = if let Some(name) = bucket_name {
                let rows = db
                    .query("SELECT id FROM buckets WHERE name = $1", &[name])
                    .await?;
                if let Some(row) = rows.first() {
                    row.get(0)
                } else {
                    return Err(BucketError::NotFound(format!(
                        "Bucket '{}' not found",
                        name
                    )));
                }
            } else {
                let current_dir = CURRENT_DIR.with(|dir| dir.clone());
                let bucket = Bucket::from_meta_data(&current_dir)?;
                bucket.id
            };

            // 2. Fetch Expectations (What this bucket expects from others or self)
            // Querying manually since Expectation model might not have this specific query logic ready or optimized for joining names.
            let expectations_rows = db
                .query(
                    "SELECT e.id, e.description, e.status, b.name 
                 FROM expectations e
                 LEFT JOIN buckets b ON e.target_bucket_id = b.id
                 WHERE e.bucket_id = $1",
                    &[&bucket_id],
                )
                .await?;

            println!(
                "\n{}",
                style("Expectations (Defined by this bucket):").bold()
            );
            if expectations_rows.is_empty() {
                println!("  (None)");
            } else {
                for row in expectations_rows {
                    let desc: String = row.get(1);
                    let status: String = row.get(2);
                    let target_name: Option<String> = row.get(3);

                    let status_styled = if status == "met" {
                        style(&status).green()
                    } else {
                        style(&status).yellow()
                    };

                    let source = if let Some(target) = target_name {
                        format!("from {}", style(target).cyan())
                    } else {
                        "(Intrinsic)".to_string()
                    };

                    println!("  [{}] {} {}", status_styled, desc, source);
                }
            }

            // 3. Fetch Pebbles (Burdens/Tasks currently in this bucket)
            let pebbles = Pebble::get_active_in_bucket(&db, bucket_id).await?;

            println!(
                "\n{}",
                style("Burden (Active Pebbles in this bucket):").bold()
            );
            if pebbles.is_empty() {
                println!("  (None)");
            } else {
                for pebble in pebbles {
                    // We want to know the origin bucket name
                    let origin_name_rows = db
                        .query(
                            "SELECT name FROM buckets WHERE id = $1",
                            &[&pebble.origin_bucket_id],
                        )
                        .await?;
                    let origin_name: String = if let Some(row) = origin_name_rows.first() {
                        row.get(0)
                    } else {
                        "Unknown".to_string()
                    };

                    println!(
                        "  - {} (Origin: {})",
                        pebble.description,
                        style(origin_name).cyan()
                    );
                }
            }
            println!("");

            Ok(())
        })
    }
}
