use crate::utils::checks::validate_path;
use clap::{Args, Parser, Subcommand, ValueEnum};
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
    Config(ConfigCommand),
    Doctor(DoctorCommand),
    Completions(CompletionsCommand),
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
            Command::Config(cmd) => &cmd.shared,
            Command::Doctor(cmd) => &cmd.shared,
            Command::Completions(cmd) => &cmd.shared,
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

    // External database configuration (required)
    #[clap(long, help = "External database host")]
    pub external_host: Option<String>,

    #[clap(long, help = "External database port", default_value = "5432")]
    pub external_port: Option<u16>,

    #[clap(long, help = "External database name", default_value = "buckets")]
    pub external_database: Option<String>,

    #[clap(long, help = "External database username")]
    pub external_username: Option<String>,

    #[clap(long, help = "External database password (optional)")]
    pub external_password: Option<String>,
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

    #[clap(required = false, help = "File path to revert (optional)")]
    pub file: Option<String>,

    #[clap(
        short = 'c',
        long = "commit",
        help = "Commit ID to revert from (defaults to most recent)"
    )]
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

    #[clap(long, help = "Test the database connection after configuration")]
    pub test_connection: bool,
}

#[derive(Subcommand, Clone)]
pub enum ConfigSubcommand {
    Get(ConfigGetCommand),
    Set(ConfigSetCommand),
    Unset(ConfigUnsetCommand),
    List(ConfigListCommand),
}

#[derive(Args, Clone)]
pub struct ConfigCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Args, Clone)]
pub struct ConfigGetCommand {
    #[clap(required = true)]
    pub key: String,

    #[clap(long, conflicts_with = "local", help = "Use global configuration")]
    pub global: bool,

    #[clap(long, conflicts_with = "global", help = "Use repository configuration")]
    pub local: bool,
}

#[derive(Args, Clone)]
pub struct ConfigSetCommand {
    #[clap(required = true)]
    pub key: String,

    #[clap(required = true)]
    pub value: String,

    #[clap(long, conflicts_with = "local", help = "Write to global configuration")]
    pub global: bool,

    #[clap(
        long,
        conflicts_with = "global",
        help = "Write to repository configuration"
    )]
    pub local: bool,
}

#[derive(Args, Clone)]
pub struct ConfigUnsetCommand {
    #[clap(required = true)]
    pub key: String,

    #[clap(long, conflicts_with = "local", help = "Unset in global configuration")]
    pub global: bool,

    #[clap(
        long,
        conflicts_with = "global",
        help = "Unset in repository configuration"
    )]
    pub local: bool,
}

#[derive(Args, Clone)]
pub struct ConfigListCommand {
    #[clap(long, conflicts_with_all = ["local", "global"], help = "Show merged config values")]
    pub effective: bool,

    #[clap(
        long,
        conflicts_with = "global",
        help = "List repository configuration"
    )]
    pub local: bool,

    #[clap(long, conflicts_with = "local", help = "List global configuration")]
    pub global: bool,
}

#[derive(Args, Clone)]
pub struct DoctorCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(long, help = "Skip database connection test")]
    pub skip_database: bool,

    #[clap(long, help = "Skip NTP server test")]
    pub skip_ntp: bool,

    #[clap(long, help = "Use repository config instead of global config")]
    pub use_repo: bool,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
}

#[derive(Args, Clone)]
pub struct CompletionsCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(value_enum)]
    pub shell: CompletionShell,
}
