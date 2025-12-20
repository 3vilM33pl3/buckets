# Buckets — Agent Guide

This file is instructions for coding agents and contributors working in this repository.

## What this repo is
- **Project type:** Rust CLI (`buckets`)
- **Primary domain:** game asset “bucket” workflow + expectation management
- **Storage:** content-addressed file storage + PostgreSQL metadata

## Quick commands (local + CI parity)
- Build (debug): `cargo build`
- Build (release, CI-style): `cargo build --release --all-features`
- Run: `cargo run -- <subcommand> [args]`
- Format: `cargo fmt`
- Lint: `cargo clippy`
- Tests (fast): `cargo test`
- Tests (CI-style): `cargo test --release`

### Targeted tests
- Single integration test file: `cargo test --test test_cli_init`
- Single test name: `cargo test test_cli_init -- --nocapture`

## Tooling prerequisites
- Rust toolchain for **edition 2021** (`Cargo.toml`).
- **Docker** is recommended for integration tests: tests use `testcontainers` to run `postgres:16-alpine`.
- Debian packaging is optional and Linux-only: see `build-deb.sh`, `build-deb-fast.sh`, `Makefile.deb`.

## Environment variables used by code/tests
- `DATABASE_URL`: PostgreSQL URL (used by `init` as a config source; set by tests when starting containers).
- `BUCKETS_SKIP_DOCKER_TESTS`, `BUCKETS_SKIP_DB_TESTS`, `NO_NETWORK`: when set to `1/true/yes` (or empty), tests that require Docker/network skip themselves (see `tests/common.rs`).
- `BUCKETS_SKIP_DB_INIT`: skips database initialization in `src/postgres_db.rs` (useful for isolated/unit testing).
- `TEST_DIR`: if set, integration tests create fixtures under this directory instead of a tempdir.

## Repository layout (high-signal)
- `src/main.rs`: entrypoint; command dispatch; decides which commands require repo bootstrap.
- `src/args.rs`: clap CLI surface area (`SharedArguments` includes `-v/--verbose` and `--json`).
- `src/commands/`: one module per subcommand; most commands implement `BucketCommand`.
- `src/commands/mod.rs`: `BucketCommand` trait + `impl_command!` helper macro.
- `src/utils/`: reusable helpers (path/security checks, config loading, compression, runtime helpers).
- `src/postgres_db.rs`: Postgres pool + migrations (`refinery` via `src/sql/migrations`).
- `src/sql/migrations/`: SQL migrations embedded into the binary.
- `tests/`: integration tests; generally one file per CLI command (`test_cli_<command>.rs`) with shared fixtures in `tests/common.rs`.
- `docs/`: user-facing and manual testing docs (command docs under `docs/commands/`).

## Code conventions and “gotchas”
### Clippy/lints
- `Cargo.toml` denies `clippy::unwrap_used`; avoid introducing `unwrap()` in non-test code.
- Prefer `Result<_, BucketError>` and propagate with `?` using `src/errors.rs`.

### Command implementation pattern
When adding or changing commands, keep the structure consistent:
1. Add/update clap args in `src/args.rs` (`Command` enum + `*Command` args struct).
2. Implement the command in `src/commands/<command>.rs` (usually `impl BucketCommand`).
3. Wire it into `src/main.rs` dispatch and `src/commands/mod.rs` module list.
4. Add/adjust the integration test in `tests/test_cli_<command>.rs`.
5. Update docs in `docs/commands/<command>.md` when behavior or flags change.

### Path safety
User-supplied paths should be validated:
- Clap parsing uses `utils/checks::validate_path` (calls `utils/security::validate_and_canonicalize_path`).
- Prefer `PathBuf` and `std::path` APIs; avoid OS-specific separators in new code.

### Database configuration (currently split)
There are multiple sources/files involved; follow existing code paths instead of assuming:
- Repo config: `.buckets/config` (TOML; includes `postgresql_connection` used by bootstrap in `src/bootstrap.rs`).
- Repo DB config: `.buckets/db_config.toml` (external connection details written by `src/database.rs` during `init`).
- Global config: `~/.buckets_config.toml` (written by `buckets setup`; loaded via `utils/config.rs`).
- Migrations: add new files to `src/sql/migrations/` with `V<N>__<name>.sql`.

Note: there is ongoing migration work; see `MIGRATION_NOTES.md` for context and known inconsistencies.

### Logging and output
- Use `log` macros (`debug!`, `info!`, …) for diagnostics; `-v/--verbose` enables debug-level logging in `src/main.rs`.
- Only some commands implement structured output via `--json` (notably `doctor`); keep output formats stable.

## CI/release notes (for changes that affect build/release)
- CI runs `cargo build --release --all-features` and `cargo test --release` (see `.github/workflows/ci.yml`).
- Coverage uses `cargo llvm-cov nextest ...` (optional locally).
- Debian package builds via `cargo deb --no-build` in CI; keep packaging files in `debian/` and scripts working when touching build metadata.

