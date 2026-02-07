// SPDX-License-Identifier: MIT

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "buckets-server", about = "REST API server for Buckets")]
pub struct ServerArgs {
    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Port to listen on
    #[arg(long, default_value_t = 3000)]
    pub port: u16,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}
