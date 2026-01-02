use crate::args::FinalizeCommand;
use crate::commands::BucketCommand;
use crate::data::bucket::{Bucket, BucketTrait};
use crate::data::pebble::Pebble;
use crate::errors::BucketError;
use crate::postgres_db::get_database;
use crate::utils::runtime::RuntimeManager;
use crate::CURRENT_DIR;
use dialoguer::console::style;
use dialoguer::{theme::ColorfulTheme, MultiSelect};

pub struct Finalize {
    args: FinalizeCommand,
}

impl BucketCommand for Finalize {
    type Args = FinalizeCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let bucket_name = &self.args.bucket;

        RuntimeManager::block_on(async {
            let db = get_database().await?;

            // 1. Resolve Bucket
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

            // 2. Fetch Active Pebbles
            let pebbles = Pebble::get_active_in_bucket(&db, bucket_id).await?;

            if pebbles.is_empty() {
                println!("{}", style("No active pebbles to resolve.").yellow());
                return Ok(());
            }

            // 3. Interactive Selection
            let pebble_descriptions: Vec<String> =
                pebbles.iter().map(|p| p.description.clone()).collect();

            let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select pebbles to resolve (Space to select, Enter to confirm):")
                .items(&pebble_descriptions)
                .interact()?; // This might need error mapping

            if selections.is_empty() {
                println!("No pebbles selected.");
                return Ok(());
            }

            // 4. Resolve selected pebbles
            for selection_idx in selections {
                let pebble = &pebbles[selection_idx];
                Pebble::resolve(&db, pebble.id).await?;
                println!("Resolved: {}", pebble.description);

                // Also update the linked Expectation if it was intrinsic?
                // Logic: If pebble marks completion of an expectation, we should update expectation status.
                // For Extrinsic: A pebble resolved in B -> Should update Expectation in A?
                // That requires more complex logic ("Notification" back to A).
                // For MVP, just resolving the pebble is good.

                // If the pebble was created via expectation and it is intrinsic (origin = current),
                // we might want to mark expectation as MET.
                if pebble.origin_bucket_id == pebble.current_bucket_id {
                    db.execute(
                        "UPDATE expectations SET status = 'met' WHERE id = $1",
                        &[&pebble.created_via_expectation_id],
                    )
                    .await?;
                }
            }

            Ok(())
        })
    }
}
