use crate::args::ExpectCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::{Bucket, BucketTrait};
use crate::data::expectation::Expectation;
use crate::data::pebble::Pebble;
use crate::errors::BucketError;
use crate::postgres_db::get_database;
use crate::utils::runtime::RuntimeManager;
use crate::CURRENT_DIR;
use log::info;
use uuid::Uuid;

pub struct Expect {
    args: ExpectCommand,
}

impl BucketCommand for Expect {
    type Args = ExpectCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let description = &self.args.description;
        let from_bucket_name = &self.args.from;
        let in_bucket_name = &self.args.in_bucket;

        RuntimeManager::block_on(async {
            let db = get_database().await?;

            // 1. Resolve Consumer Bucket (where the expectation is held)
            let consumer_bucket_id = if let Some(name) = in_bucket_name {
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
                // Determine from current directory
                let current_dir = CURRENT_DIR.with(|dir| dir.clone());
                let bucket = Bucket::from_meta_data(&current_dir)?;
                bucket.id
            };

            // 2. Resolve Producer Bucket (where the expectation is directed)
            let producer_bucket_id = if let Some(name) = from_bucket_name {
                let rows = db
                    .query("SELECT id FROM buckets WHERE name = $1", &[name])
                    .await?;
                if let Some(row) = rows.first() {
                    Some(row.get::<usize, Uuid>(0))
                } else {
                    return Err(BucketError::NotFound(format!(
                        "Source bucket '{}' not found",
                        name
                    )));
                }
            } else {
                None
            };

            // 3. Create Expectation Record
            let expectation = Expectation::create(
                &db,
                consumer_bucket_id,
                producer_bucket_id,
                description.clone(),
            )
            .await?;

            info!("Expectation created: {}", expectation.id);

            // 4. Create Pebble Logic
            if let Some(producer_id) = producer_bucket_id {
                // Extrinsic Expectation
                // Create Transport Pebble in Producer
                // Origin = Consumer (The demand originates here)
                // Current = Producer (The burden is placed here)
                let pebble = Pebble::create(
                    &db,
                    description.clone(),
                    consumer_bucket_id, // Origin
                    producer_id,        // Current
                    None,               // Original Pebble
                    expectation.id,
                )
                .await?;

                info!(
                    "Created extrinsic pebble in bucket {}: {}",
                    producer_id, pebble.id
                );
            } else {
                // Intrinsic Expectation
                // Create Pebble in Consumer
                // Origin = Consumer
                // Current = Consumer
                let pebble = Pebble::create(
                    &db,
                    description.clone(),
                    consumer_bucket_id,
                    consumer_bucket_id,
                    None,
                    expectation.id,
                )
                .await?;
                info!(
                    "Created intrinsic pebble in bucket {}: {}",
                    consumer_bucket_id, pebble.id
                );
            }

            println!("Expectation recorded.");
            Ok(())
        })
    }
}
