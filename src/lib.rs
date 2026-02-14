// SPDX-License-Identifier: MIT
#![cfg_attr(test, allow(clippy::unwrap_used))]

//! Buckets library - shared types and database access for the buckets CLI and TUI.

pub mod bootstrap;
pub mod config;
pub mod data;
pub mod database;
pub mod errors;
pub mod postgres_db;
pub mod utils;
pub mod world;

use std::path::PathBuf;

thread_local! {
    /// Current working directory, shared across CLI and TUI binaries.
    pub static CURRENT_DIR: PathBuf = std::env::current_dir().unwrap_or_else(|_| {
        eprintln!("Error: Failed to get current directory. Using current directory as fallback.");
        PathBuf::from(".")
    });
}
