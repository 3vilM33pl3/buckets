use crate::utils::checks::validate_path;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Command {
    // Repository commands
    Init(InitCommand),
    Create(CreateCommand),
    Commit(CommitCommand),
    Revert(RevertCommand),
    Rollback(RollbackCommand),
    Stash(StashCommand),
    // Information commands
    Status(StatusCommand),
    History(HistoryCommand),
    List(ListCommand),
    Stats(StatsCommand),
    // Expectation commands
    Expect(ExpectCommand),
    Check(CheckCommand),
    Link(LinkCommand),
    Finalize(FinalizeCommand),
    Schema(SchemaCommand),
    // Global commands
    Setup(SetupCommand),
}

impl Command {
    pub fn shared_args(&self) -> &SharedArguments {
        match self {
            Command::Init(cmd) => &cmd.shared,
            Command::Create(cmd) => &cmd.shared,
            Command::Commit(cmd) => &cmd.shared,
            Command::Revert(cmd) => &cmd.shared,
            Command::Rollback(cmd) => &cmd.shared,
            Command::Stash(cmd) => &cmd.shared,
            Command::Status(cmd) => &cmd.shared,
            Command::History(cmd) => &cmd.shared,
            Command::List(cmd) => &cmd.shared,
            Command::Stats(cmd) => &cmd.shared,
            Command::Expect(cmd) => &cmd.shared,
            Command::Check(cmd) => &cmd.shared,
            Command::Link(cmd) => &cmd.shared,
            Command::Finalize(cmd) => &cmd.shared,
            Command::Schema(cmd) => &cmd.shared,
            Command::Setup(cmd) => &cmd.shared,
        }
    }
}

#[derive(Parser)]
#[clap(
    name = "buckets",
    version = env!("CARGO_PKG_VERSION"),
    author("3vilM33pl3"),
)]
pub struct CliArguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Default, Debug, Clone)]
pub struct SharedArguments {
    #[clap(short, long)]
    pub verbose: bool,
    
    #[clap(long)]
    pub json: bool,
}

#[derive(Args, Clone)]
pub struct InitCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(required = true)]
    pub repo_name: String,

    #[clap(long, default_value = "embedded", value_parser = validate_database_type)]
    pub database: String,
}

fn validate_database_type(s: &str) -> Result<String, String> {
    match s.to_lowercase().as_str() {
        "embedded" | "external" | "postgresql" | "postgres" => Ok(s.to_string()),
        _ => Err(format!(
            "Invalid database type '{}'. Valid options are: embedded, external, postgresql",
            s
        )),
    }
}

#[derive(Args, Clone)]
pub struct CreateCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(required = true)]
    pub bucket_name: String,
}

#[derive(Args, Clone)]
pub struct CommitCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(required = true)]
    pub message: String,
}

#[derive(Args, Clone)]
pub struct RevertCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(required = true, help = "File path to revert")]
    pub file: String,
    
    #[clap(short = 'c', long = "commit", help = "Commit ID to revert from (defaults to most recent)")]
    pub commit_id: Option<String>,
}

#[derive(Args, Clone)]
pub struct RollbackCommand {
    #[clap(short, long, value_name = "PATH", value_parser = validate_path)]
    pub path: Option<PathBuf>,

    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
pub struct StashCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
pub struct StatusCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Debug, Clone)]
pub struct HistoryCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
pub struct ListCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
pub struct StatsCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
pub struct ExpectCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
pub struct CheckCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
pub struct LinkCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
pub struct FinalizeCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
pub struct SchemaCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
pub struct SetupCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}
