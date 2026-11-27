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

            let json = self.args.shared.json;
            let print_resolved = |resolved: &[serde_json::Value]| {
                println!("{}", serde_json::json!({ "resolved": resolved }));
            };

            if pebbles.is_empty() {
                if json {
                    print_resolved(&[]);
                } else {
                    println!("{}", style("No active pebbles to resolve.").yellow());
                }
                return Ok(());
            }

            // 3. Selection: flags for scripts and agents, a menu otherwise.
            let selections: Vec<usize> = if self.args.all {
                (0..pebbles.len()).collect()
            } else if !self.args.pebbles.is_empty() {
                let mut picked = Vec::new();
                for wanted in &self.args.pebbles {
                    let index = pebbles
                        .iter()
                        .position(|p| p.id.to_string() == *wanted || p.description == *wanted)
                        .ok_or_else(|| {
                            BucketError::NotFound(format!("No active pebble matches '{}'", wanted))
                        })?;
                    if !picked.contains(&index) {
                        picked.push(index);
                    }
                }
                picked
            } else if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                return Err(BucketError::from(
                    "No terminal to prompt on. Use --all or --pebble <id or description>.",
                ));
            } else {
                let pebble_descriptions: Vec<String> =
                    pebbles.iter().map(|p| p.description.clone()).collect();
                MultiSelect::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select pebbles to resolve (Space to select, Enter to confirm):")
                    .items(&pebble_descriptions)
                    .interact()?
            };

            if selections.is_empty() {
                if json {
                    print_resolved(&[]);
                } else {
                    println!("No pebbles selected.");
                }
                return Ok(());
            }

            // 4. Resolve selected pebbles. With --json, one document is printed
            // at the end: {"resolved": [{"id", "description"}, ...]}.
            let mut resolved = Vec::new();
            for selection_idx in selections {
                let pebble = &pebbles[selection_idx];
                Pebble::resolve(&db, pebble.id).await?;
                if json {
                    resolved.push(serde_json::json!({
                        "id": pebble.id.to_string(),
                        "description": pebble.description,
                    }));
                } else {
                    println!("Resolved: {}", pebble.description);
                }

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

            if json {
                print_resolved(&resolved);
            }
            Ok(())
        })
    }
}
