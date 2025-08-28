# GitHub Copilot Instructions for Buckets

## Project Overview

**Buckets** is a CLI tool for game asset and expectation management written in Rust. It controls versions of work and sets expectations between collaborators through a bucket-based workflow where each stage contains resources for creating game assets at specific production pipeline stages.

### Repository Information
- **Language**: Rust (edition 2021)
- **Size**: ~50 source files, medium complexity
- **Target**: CLI application for asset version management
- **Database**: PostgreSQL (migrating from DuckDB) 
- **Key Dependencies**: clap, tokio, postgres, blake3, zstd
- **MSRV**: Rust 1.70+

## Build and Validation Commands

### Prerequisites
```bash
# Required: Rust 1.70+ with Cargo
rustup default stable  # or nightly-2025-01-15 for some features
```

### Build Commands (Always Run in Order)
```bash
# 1. ALWAYS clean build first - reuses artifacts efficiently
cargo build

# 2. Release build (for performance testing)
cargo build --release

# 3. Format code (REQUIRED before commits)
cargo fmt

# 4. Lint code (fix warnings when possible) 
cargo clippy --fix --allow-dirty --allow-staged
cargo clippy
```

**Critical**: Run `cargo fmt` before any commit - the CI fails on formatting issues.

### Test Commands (Currently Broken - See Workarounds)
```bash
# Standard tests (currently fail due to import issues)
cargo test

# Coverage tests (requires additional tools)
cargo install cargo-llvm-cov cargo-nextest
cargo llvm-cov nextest --all-features --workspace --lcov --output-path lcov.info
```

**Known Test Issues**: Tests currently fail due to:
1. Missing `use serial_test::serial;` imports  
2. Missing `use std::fs::File;` imports
3. Missing `use predicates::str::contains;` imports

**Workaround**: Add these imports when fixing test files.

### Manual Testing Workflow
```bash
# Build and test basic functionality
cargo build
./target/debug/buckets --help

# Note: Full workflow testing requires network access for PostgreSQL setup
# The init command may fail in CI environments with:
# "Failed to install PostgreSQL: HTTP status client error (403 Forbidden)"
# This is expected in sandboxed environments - focus on code changes rather than manual testing

# Basic command testing (network-independent):
./target/debug/buckets init --help
./target/debug/buckets create --help  
./target/debug/buckets commit --help
```

### Debian Package Building
```bash
# Fast incremental build (recommended for development)
./build-deb-fast.sh

# Full clean build (first time or release)
./build-deb.sh

# Or using Make
make -f Makefile.deb deb
```

**Prerequisites for Debian build**:
```bash
sudo apt-get install dpkg-dev debhelper devscripts cargo rustc pkg-config libssl-dev
```

## Project Layout and Architecture

### Core Source Structure
```
src/
├── main.rs              # Entry point, command dispatch
├── args.rs              # CLI argument parsing (clap)
├── errors.rs            # Centralized error handling (BucketError)
├── commands/            # Command implementations
│   ├── init.rs          # Initialize repositories  
│   ├── create.rs        # Create buckets
│   ├── commit.rs        # Handle commits
│   ├── status.rs        # Repository status
│   └── ...
├── data/                # Data structures (Bucket, Commit)
├── utils/               # Utility functions
│   ├── checks.rs        # Directory/repository validation
│   ├── compression.rs   # zstd compression for files
│   └── utils.rs         # General utilities
├── postgres_db.rs       # PostgreSQL database operations
├── database.rs          # Database abstraction layer  
└── sql/                 # Database schema and migrations
    ├── schema.sql       # PostgreSQL table definitions
    └── migrations/      # Database migration files
```

### Configuration Files
- `Cargo.toml` - Rust dependencies and project config
- `.vscode/settings.json.template` - VSCode development settings
- `.github/workflows/ci.yml` - Main CI pipeline
- `Makefile.deb` - Debian packaging configuration

### Database Schema
PostgreSQL tables:
- `buckets` - Bucket metadata (id, name, path)
- `commits` - Commit records (id, bucket_id, message, timestamp)  
- `files` - File tracking (id, commit_id, file_path, blake3_hash)

### Repository Structure
```
./repo_name/
├── .buckets/            # Repository metadata
│   ├── config           # Repository configuration
│   ├── buckets.db       # PostgreSQL database (embedded)
│   └── database_type    # Database type marker
└── bucket_name/         # Individual buckets
    ├── .b/              # Bucket metadata
    │   ├── info         # Bucket configuration
    │   └── storage/     # Compressed file storage
    └── [user files]     # Working files
```

## Continuous Integration and Validation

### GitHub Actions Workflows
1. **CI Pipeline** (`.github/workflows/ci.yml`):
   - Runs on: push to main, pull requests
   - Steps: Build → Test → Lint → Coverage → Upload to Codecov
   - **Critical**: Uses `self-hosted` runner, may have different environment

2. **Debian Package** (`.github/workflows/debian-package.yml`):
   - Triggers: tags (v*), workflow_dispatch, debian/ changes
   - Uses nightly Rust: `nightly-2025-01-15`

### Pre-commit Validation
**ALWAYS run before committing**:
```bash
cargo fmt                              # Fix formatting
cargo clippy --fix --allow-dirty      # Fix linting issues  
cargo build                           # Ensure builds
cargo test                            # Run tests (when fixed)
```

### Known CI Pitfalls
1. **Formatting**: CI fails immediately on `cargo fmt --check` errors
2. **Test Dependencies**: Some tests require network access and may fail in CI
3. **Database Migration**: Code has warnings due to incomplete PostgreSQL migration  
4. **Nightly Features**: Some features require nightly Rust toolchain
5. **Network Dependencies**: PostgreSQL installation may fail with 403 Forbidden errors in restricted environments

## Common Development Tasks

### Adding New Commands
1. Create new file in `src/commands/`
2. Implement `BucketCommand` trait with `execute()` method
3. Add to `args.rs` enum and parser
4. Register in `main.rs` dispatch function
5. Add tests in `tests/test_cli_[command].rs`

### Database Operations
- Use `get_database().await?` for PostgreSQL connections
- All database operations are async - wrap in `tokio::runtime`
- Use prepared statements with `$1, $2` parameter syntax
- UUID generation via PostgreSQL's `uuid_generate_v4()`

### File Operations  
- Files hashed with BLAKE3 algorithm
- Compressed with zstd before storage in `.b/storage/`
- Use utilities in `src/utils/compression.rs`

### Testing
- Tests in `tests/` directory, one per command
- Use `#[serial]` for tests requiring sequential execution
- Common test utilities in `tests/common.rs`
- Use `tempfile` crate for isolated test environments

## Troubleshooting Guide

### Build Failures
1. **"serial not found"**: Add `use serial_test::serial;` to test files
2. **"File not found"**: Add `use std::fs::File;` to test files  
3. **"contains not found"**: Add `use predicates::str::contains;`
4. **Formatting errors**: Run `cargo fmt` before committing
5. **Trailing whitespace**: Use `sed -i 's/[[:space:]]*$//' filename` to remove

### Database Issues
- Embedded PostgreSQL installs automatically on first run
- Connection pooling via `deadpool-postgres`
- Migration state tracked in `MIGRATION_NOTES.md`

### Performance Issues
- Use `--release` builds for performance testing
- Database operations are async - avoid blocking calls
- File compression may be CPU intensive for large assets

### Environment Setup
```bash
# VSCode development setup
cp .vscode/settings.json.template .vscode/settings.json

# Install development tools
cargo install cargo-llvm-cov cargo-nextest

# For Debian packaging
sudo apt-get install dpkg-dev debhelper devscripts pkg-config libssl-dev
```

## Important: Trust These Instructions

**These instructions are comprehensive and tested.** Only search for additional information if:
1. The instructions are incomplete for your specific task
2. You encounter errors not covered in the troubleshooting section  
3. The repository structure has changed significantly

Most common development tasks and build issues are covered above. Following these instructions will minimize exploration time and reduce failed command attempts.