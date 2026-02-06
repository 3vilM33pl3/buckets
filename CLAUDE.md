# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About Buckets

Buckets is a CLI tool for game asset and expectation management. It controls versions of work and sets/records expectations between collaborators. Each stage of the workflow is represented by a bucket containing resources to create game assets at specific production pipeline stages.

## Development Commands

### Building and Testing
- `cargo build` - Build the project
- `cargo build --release` - Build optimized release version
- `cargo test` - Run all tests
- `cargo test --release` - Run tests in release mode
- `cargo clippy` - Run linting checks (unwrap_used is denied)
- `cargo fmt` - Format code

### Running Single Tests
```bash
# Run a specific test file
cargo test --test test_cli_init

# Run a specific test function
cargo test test_cli_init

# Run with output visible (for debugging)
cargo test test_cli_init -- --nocapture

# Run ignored tests
cargo test -- --ignored
```

### Advanced Testing
- `cargo llvm-cov nextest --all-features --workspace --lcov --output-path lcov.info` - Generate code coverage report (requires cargo-llvm-cov and cargo-nextest)
- Tests use `#[serial]` from the `serial_test` crate for tests that need sequential execution
- Some tests use `#[ignore]` and must be explicitly run with `-- --ignored`

### Environment Variables for Tests
Skip Docker-dependent tests when Docker is unavailable:
- `BUCKETS_SKIP_DOCKER_TESTS=1` - Skip tests requiring Docker
- `BUCKETS_SKIP_DB_TESTS=1` - Skip database tests
- `NO_NETWORK=1` - Skip tests requiring network

## Architecture Overview

### Command Structure and Pattern
All commands follow a consistent trait-based pattern:

1. **Command Definition**: Each subcommand is defined as a struct in [args.rs](src/args.rs) (e.g., `InitCommand`, `CreateCommand`)
2. **Command Implementation**: Implementation lives in `src/commands/<command>.rs`
3. **BucketCommand Trait**: All commands implement the `BucketCommand` trait with:
   - `type Args` - The command's argument type
   - `fn new(args: &Self::Args) -> Self` - Constructor
   - `fn execute(&self) -> Result<(), BucketError>` - Execution logic
4. **Dispatch**: [main.rs](src/main.rs) dispatches commands via pattern matching on the `Command` enum

Example command structure:
```rust
pub struct MyCommand {
    args: MyCommandArgs,
}

impl BucketCommand for MyCommand {
    type Args = MyCommandArgs;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        // Implementation
        Ok(())
    }
}
```

### Key Components
- **[args.rs](src/args.rs)**: CLI argument parsing using clap with `SharedArguments` (verbose, json flags) common to all commands
- **[errors.rs](src/errors.rs)**: Centralized error handling with `BucketError` enum using thiserror
- **[world.rs](src/world.rs)**: Global state management - tracks working directory, repo root, database path, active bucket, and verbose flag
- **[commands/mod.rs](src/commands/mod.rs)**: Defines `BucketCommand` trait and optional `CommandDispatcher` for centralized execution
- **utils/**: Reusable functions (path validation, security checks, compression, directory validation)
- **data/**: Core data structures (`Bucket`, `Commit`) with trait-based interfaces

### Configuration System
Buckets has a two-tier configuration system:

1. **Global Configuration** (`~/.buckets_config.toml`):
   - Managed via `buckets setup` command
   - Contains PostgreSQL connection strings and NTP server settings
   - Inherited by new repositories

2. **Repository Configuration** (`.buckets/config`):
   - Created during `buckets init`
   - Inherits from global config
   - Can override global settings

### Database & Storage
- **PostgreSQL** with **pgvector** extension for data persistence and semantic search
- Schema in `src/postgres_db.rs`, migrations in `src/sql/migrations/`
- File storage: Content-addressable in `.b/storage/` within each bucket
- File hashing: BLAKE3 for content integrity
- Compression: zstd for efficient storage
- UUID-based object identification

### Semantic Search (pgvector)
- Expectations use vector embeddings for duplicate detection
- Embedding model: `all-MiniLM-L6-v2` (384 dimensions) via Candle
- Model is lazily loaded and cached globally in `src/utils/embeddings.rs`
- First run downloads ~90MB model from HuggingFace Hub
- HNSW index for fast cosine similarity search (>85% threshold warns for duplicates)

### Thread-Local State
Defined in [main.rs](src/main.rs):
- `CURRENT_DIR`: Current working directory (used throughout the codebase)
- `EXIT`: Program exit code tracking
- Access via `CURRENT_DIR.with(|dir| dir.clone())`

### Error Handling
All errors use the centralized `BucketError` enum with:
- `From<io::Error>` implementation for seamless propagation with `?` operator
- `From<tokio_postgres::Error>` for database errors
- Custom error variants for domain-specific errors

### Logging
- Uses `env_logger` with configurable verbosity
- Default: warnings and errors only
- `-v` flag enables debug-level logging
- Initialize via `init_logging(verbose)` in [main.rs](src/main.rs:47)

## Testing Structure
- Integration tests in `tests/` directory
- One test file per command (e.g., `test_cli_init.rs`, `test_cli_commit.rs`)
- Common test utilities in [tests/common.rs](tests/common.rs)
- Uses `tempfile` crate for isolated test environments
- Uses `serial_test` crate with `#[serial]` for tests requiring sequential execution
- Uses `assert_cmd` for CLI testing
- Uses `testcontainers` with `pgvector/pgvector:pg16` Docker image for database tests

Test naming convention: `test_cli_<command>`

### Test Fixtures
- `TestDatabase` - Spins up a PostgreSQL container with pgvector, sets `DATABASE_URL` env var, auto-cleans on drop
- `RepoFixture` - Creates a complete test repository with initialized database and bucket

## Project Structure
```
buckets/
├── src/
│   ├── args.rs              # CLI argument definitions
│   ├── main.rs              # Entry point, command dispatch
│   ├── errors.rs            # Error types
│   ├── world.rs             # Global state
│   ├── postgres_db.rs       # Database connection and operations
│   ├── commands/            # Command implementations
│   │   ├── mod.rs          # BucketCommand trait
│   │   ├── init.rs, create.rs, commit.rs, expect.rs, ...
│   ├── data/                # Core data structures (Bucket, Commit)
│   ├── utils/               # Utility functions (compression, security, embeddings)
│   └── sql/migrations/      # PostgreSQL schema migrations (V1, V2, V3)
├── tests/                   # Integration tests (test_cli_*.rs)
│   └── common.rs           # Test fixtures (TestDatabase, RepoFixture)
└── debian/                  # Debian packaging
```

## Key Implementation Details

### Repository Structure
A buckets repository has this structure:
```
repo_name/
├── .buckets/
│   ├── config              # Repository configuration (TOML)
│   └── database_type       # "PostgreSQL" marker file
└── bucket_name/            # Individual buckets
    └── .b/
        ├── info            # Bucket metadata (TOML: id, name, relative_bucket_path)
        └── storage/        # Compressed file storage (content-addressable)
```

### Static Arguments
The `ARGS` static in [main.rs](src/main.rs:23) is initialized lazily using `once_cell::Lazy` to parse CLI arguments once and make them globally available.

### Debian Packaging
- Build scripts: `build-deb.sh` (clean build) and `build-deb-fast.sh` (incremental)
- Package files in `debian/` directory
- Makefile at `Makefile.deb` for package building

### Diagnostics
- `buckets doctor` - System diagnostics command that tests database connectivity, NTP server, and pgvector availability
- Useful for troubleshooting setup issues
