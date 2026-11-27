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
    // File locks for unmergeable assets
    Lock(LockCommand),
    Unlock(UnlockCommand),
    Locks(LocksCommand),
    // Sharing objects with a remote store (e.g. a NAS mount)
    Remote(RemoteCommand),
    Push(PushCommand),
    Pull(PullCommand),
    // Information commands
    Status(StatusCommand),
    History(HistoryCommand),
    List(ListCommand),
    Stats(StatsCommand),
    // Expectation commands
    Expect(ExpectCommand),
    Check(CheckCommand),
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
            Command::Lock(cmd) => &cmd.shared,
            Command::Unlock(cmd) => &cmd.shared,
            Command::Locks(cmd) => &cmd.shared,
            Command::Remote(cmd) => &cmd.shared,
            Command::Push(cmd) => &cmd.shared,
            Command::Pull(cmd) => &cmd.shared,
            Command::Status(cmd) => &cmd.shared,
            Command::History(cmd) => &cmd.shared,
            Command::List(cmd) => &cmd.shared,
            Command::Stats(cmd) => &cmd.shared,
            Command::Expect(cmd) => &cmd.shared,
            Command::Check(cmd) => &cmd.shared,
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
    #[clap(short, long, global = true, help = "Show debug logging")]
    pub verbose: bool,

    #[clap(long, global = true, help = "Print machine-readable JSON")]
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
#[command(about = "Set uncommitted changes aside and restore the last commit")]
pub struct StashCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[command(subcommand)]
    pub action: Option<StashAction>,
}

#[derive(Subcommand, Clone, Debug)]
pub enum StashAction {
    /// Save uncommitted changes and restore the last commit (the default)
    Save {
        #[clap(short, long, help = "Description of the stash")]
        message: Option<String>,
    },
    /// Re-apply the most recent stash and remove it
    Pop {
        #[clap(long, help = "Apply even if the bucket has uncommitted changes")]
        force: bool,
    },
    /// List stashes, newest first
    List,
    /// Delete the most recent stash without applying it
    Drop,
}

#[derive(Args, Clone)]
#[command(about = "Lock files so nobody else can commit changes to them")]
pub struct LockCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(required = true, help = "Files to lock, relative to the bucket")]
    pub files: Vec<String>,
}

#[derive(Args, Clone)]
#[command(about = "Release file locks")]
pub struct UnlockCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(required = true, help = "Files to unlock, relative to the bucket")]
    pub files: Vec<String>,

    #[clap(long, help = "Break a lock held by someone else")]
    pub force: bool,
}

#[derive(Args, Clone)]
#[command(about = "List locked files in the current bucket")]
pub struct LocksCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
#[command(about = "Configure the remote object store (a directory such as a NAS mount)")]
pub struct RemoteCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[command(subcommand)]
    pub action: RemoteAction,
}

#[derive(Subcommand, Clone, Debug)]
pub enum RemoteAction {
    /// Use a directory as the remote store
    Set { path: PathBuf },
    /// Show the configured remote store
    Show,
    /// Remove the remote store setting
    Unset,
}

#[derive(Args, Clone)]
#[command(about = "Copy committed objects to the remote store")]
pub struct PushCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,
}

#[derive(Args, Clone)]
#[command(
    about = "Update the working files to the latest commit, fetching objects from the remote store"
)]
pub struct PullCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(long, help = "Overwrite local edits that are not committed")]
    pub force: bool,
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

    #[clap(help = "Description of the expectation")]
    pub description: String,

    #[clap(long, help = "Bucket to expect from (optional)")]
    pub from: Option<String>,

    #[clap(
        long,
        help = "Bucket to add expectation to (optional, defaults to current)"
    )]
    pub in_bucket: Option<String>,

    #[clap(long, help = "Skip the duplicate check (no embedding model download)")]
    pub no_similarity: bool,
}

#[derive(Args, Clone)]
pub struct CheckCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(help = "Bucket to check (optional, defaults to current)")]
    pub bucket: Option<String>,
}

#[derive(Args, Clone)]
pub struct FinalizeCommand {
    #[clap(flatten)]
    pub shared: SharedArguments,

    #[clap(help = "Bucket to finalize (optional, defaults to current)")]
    pub bucket: Option<String>,

    #[clap(long, help = "Resolve all active pebbles without prompting")]
    pub all: bool,

    #[clap(
        long = "pebble",
        value_name = "ID_OR_DESCRIPTION",
        help = "Resolve a specific pebble without prompting (repeatable)"
    )]
    pub pebbles: Vec<String>,
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

    #[clap(short, long, help = "Output file path")]
    pub output: Option<PathBuf>,

    #[clap(
        short,
        long,
        conflicts_with = "output",
        help = "Automatically install to standard shell directory (Bash, Zsh, Fish only)"
    )]
    pub install: bool,
}
