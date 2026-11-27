[![Rust](https://github.com/3vilM33pl3/buckets/actions/workflows/ci.yml/badge.svg)](https://github.com/3vilM33pl3/buckets/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/3vilM33pl3/buckets/graph/badge.svg?token=Kolp8rdCwL)](https://codecov.io/gh/3vilM33pl3/buckets)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

# Buckets

A CLI tool for game asset and expectation management that controls versions of work and sets/records expectations between collaborators.

## Overview

Buckets is a version control and workflow management tool designed specifically for game asset creation. Every stage of the workflow is represented by a bucket containing all the resources needed to create game assets at specific production pipeline stages.

### Key Features

- **Version Control**: Track and manage multiple versions of game assets
- **Bucket-based Workflow**: Organize work into buckets representing different pipeline stages
- **Expectation Management**: Define and check requirements between workflow stages
- **File Integrity**: BLAKE3 hashing ensures file integrity
- **Compression**: Built-in zstd compression for efficient storage
- **Semantic Search**: AI-powered duplicate detection for expectations using `pgvector`
- **Database Backend**: PostgreSQL for reliable data persistence

### How It Works

The workflow is represented by linking buckets together through expectations, where the output of one bucket becomes the input of another. Expectations are simple rules that indicate what a bucket needs to finalize its work.

#### Example Workflow

Consider creating a 3D model for a game that needs concept art and textures:

**Bucket 1: Concept Art**
- Expectations:
  1. Has a mood board
  2. Has concept art
  3. Concept art is approved by art director

**Bucket 2: 3D Model**
- Expectations:
  1. Has concept art for the model
  2. Has a 3D model
  3. Has textures for the model

Once the concept art is ready and approved, you finalize the bucket. The 3D model bucket automatically receives the latest version of the concept art, satisfying its first expectation.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/3vilM33pl3/buckets.git
cd buckets

# Build with Cargo
cargo build --release

# The binary will be at target/release/buckets
```

### Debian Package

Build and install as a Debian package for system-wide installation:

```bash
# Install build dependencies
sudo apt-get install dpkg-dev debhelper devscripts cargo rustc pkg-config libssl-dev

# Build the package (choose one method)
./build-deb.sh                    # Full clean build (slower, first time)
./build-deb-fast.sh               # Faster incremental build
make -f Makefile.deb deb          # Using the Makefile
dpkg-buildpackage -us -uc -b     # Using dpkg directly

# Install the package
sudo dpkg -i ../buckets_*.deb

# If there are dependency issues
sudo apt-get install -f
```

**Note**: The first build will take several minutes to download and compile all Rust dependencies. Subsequent builds using `build-deb-fast.sh` will be much faster as they reuse build artifacts.

To uninstall:
```bash
sudo apt-get remove buckets       # Remove package
sudo apt-get purge buckets        # Remove package and config files
```

## Commands

### Global Configuration

#### `buckets setup`
Configure global settings for all Buckets repositories on your system.

```bash
buckets setup
```

This interactive command allows you to configure:
- PostgreSQL connection strings for external databases
- Custom NTP servers for time synchronization
- Global defaults inherited by new repositories

See [Setup Command Documentation](docs/setup_command.md) for detailed usage.

### buckets-server database TLS/mTLS environment

`buckets-server` always requires `DATABASE_URL` and can optionally use certificate-based TLS:

- `DATABASE_URL=postgresql://buckets_server@10.22.6.42:5432/buckets`
- `BUCKETS_DB_TLS_CA_CERT_PATH=/etc/buckets/certs/root_ca.crt`
- `BUCKETS_DB_TLS_CLIENT_CERT_PATH=/etc/buckets/certs/buckets_server.crt`
- `BUCKETS_DB_TLS_CLIENT_KEY_PATH=/etc/buckets/certs/buckets_server.key`

Notes:

- Set only `BUCKETS_DB_TLS_CA_CERT_PATH` for server-verified TLS.
- Set all three TLS variables for mTLS.
- If only one of client cert/key is set, `buckets-server` exits with a validation error.

### Repository Management

#### `buckets init <repo_name>`
Initialize a new buckets repository.

```bash
buckets init my_game_assets
```

### Bucket Operations

#### `buckets create <bucket_name>`
Create a new bucket for content.

```bash
buckets create concept_art
```

#### `buckets list`
List all buckets in the repository.

```bash
buckets list
```

### Version Control

#### `buckets commit <message>`
Save the current state of a bucket with a descriptive message.

```bash
buckets commit "Added character concept sketches"
```

#### `buckets history`
Show the commit history for the current bucket.

```bash
buckets history
```

#### `buckets status`
Show which files have changed since the last commit.

```bash
buckets status
```

#### `buckets stats`
Display statistics about the bucket and repository.

```bash
buckets stats
```

### File Management

#### `buckets revert <file>`
Restore a specific file from a previous commit.

```bash
# Restore from the most recent commit
buckets revert assets/model.blend

# Restore from a specific commit
buckets revert assets/model.blend --commit abc123def
```

#### `buckets rollback [--path <file>]`
Discard uncommitted changes and restore files to their state in the last commit.

```bash
# Rollback all changes
buckets rollback

# Rollback a specific file
buckets rollback --path assets/texture.png
```

#### `buckets stash`
Set uncommitted changes aside and put the bucket back to its last commit.
Stashes are kept per bucket in `.b/stash.json`, with contents in the shared object store.

```bash
buckets stash                   # same as: buckets stash save
buckets stash save -m "wip"
buckets stash list
buckets stash pop               # re-apply the newest stash
buckets stash drop
```

### Locks
Lock files that cannot be merged (`.uasset`, `.umap`, `.blend`, ...). While a file is
locked, `buckets commit` refuses changes to it from anyone except the lock owner.
The owner is `BUCKETS_USER` when set (useful for agents), otherwise `user@host`.

```bash
buckets lock Maps/L_Island.umap
buckets locks
buckets unlock Maps/L_Island.umap
buckets unlock --force Maps/L_Island.umap   # break someone else's lock
```

### Sharing between machines
Commit metadata lives in the shared PostgreSQL database; file contents travel through a
remote object store, which is any directory (for example a NAS mount).

```bash
buckets remote set /mnt/nas/buckets-objects   # stored in .buckets/remote.toml
buckets push                                  # copy committed objects the remote lacks
buckets pull                                  # bring working files up to the latest commit
buckets pull --force                          # also overwrite local, uncommitted edits
```

`BUCKETS_REMOTE` overrides the configured path. `pull` only fetches the objects the latest
commit needs, keeps untracked files, and never overwrites a local edit without `--force`.

### Expectations and Workflow

#### `buckets expect`
Define what this bucket expects from other buckets.

```bash
buckets expect
```

#### `buckets check`
Verify if all expectations are met.

```bash
buckets check
```

#### `buckets finalize`
Resolve active pebbles in a bucket. Prompts with a menu in a terminal; scripts and
agents pass `--all` or `--pebble <id or description>` (repeatable).

```bash
buckets finalize
buckets finalize --all
buckets finalize --pebble "Concept art is approved"
```

### Scripting and agents
Every command accepts `--json` (before or after its arguments) and prints
machine-readable output, for example `buckets status --json` or `buckets commit "msg" --json`.
`buckets expect --no-similarity` (or `BUCKETS_NO_EMBEDDINGS=1`) skips the duplicate check and
the embedding model download.

### Database Operations

#### `buckets schema`
Display or manage the database schema.

```bash
buckets schema
```

## Development

### Prerequisites

- Rust 1.70 or later
- Cargo
- PostgreSQL with `pgvector` extension installed

### Building from Source

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with verbose logging
buckets -v <command>
```

### VSCode Setup

For VSCode development:

```bash
# Copy the settings template
cp .vscode/settings.json.template .vscode/settings.json

# Adjust rust-analyzer settings to match your toolchain
```

### Project Structure

```
buckets/
├── src/
│   ├── commands/     # Command implementations
│   ├── data/         # Data structures
│   ├── utils/        # Utility functions
│   └── main.rs       # Entry point
├── tests/            # Integration tests
├── docs/             # Documentation
└── debian/           # Debian packaging files
```

## Technical Details

- **Storage**: File contents are stored once per repository, zstd-compressed and named by their
  BLAKE3 hash, in `.buckets/objects/<2 hex>/<rest>`. Identical files in different buckets are
  stored once. Writes go to a temporary file, are fsynced and then renamed, so a crash never
  leaves a half-written object. Repositories made by older versions kept objects in each
  bucket's `.b/storage/`; those are still read.
- **Commits**: Each commit records a full manifest (every file and its hash), so unchanged and
  deleted files are tracked exactly. The commit and its file rows are written in one database
  transaction after the objects are stored. Commits made by versions before 0.5 only recorded
  changed files; their history is read by folding earlier commits together.
- **Status cache**: `.b/index.json` caches each file's size, modification time and hash, so
  `status` and `commit` only re-hash files that changed. Hashing uses a 1 MiB buffer and all CPU cores.
- **Database**: PostgreSQL stores metadata, commit history, locks and expectations.
  `pgvector` is optional: without it, duplicate-expectation search is turned off.
- **Credentials**: Database passwords are never written into the repository. They come from
  `DATABASE_URL`, `BUCKETS_DB_PASSWORD`, the global config, or `~/.buckets_credentials.toml`
  (mode 0600; override the path with `BUCKETS_CREDENTIALS_FILE`). `buckets init --external-password`
  stores the password there.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This work is dual-licensed under Apache 2.0 and MIT.
You can choose between one of them if you use this work.

`SPDX-License-Identifier: Apache-2.0 OR MIT`
