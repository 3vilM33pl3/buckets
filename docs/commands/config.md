# `buckets config` Command

Manage Buckets configuration values without manually editing TOML files. The command supports global and repository scopes and uses TOML tables for categories.

## Usage

```bash
buckets config get <key> [--global|--local]
buckets config set <key> <value> --global|--local
buckets config unset <key> --global|--local
buckets config list [--global|--local|--effective]
```

## Scopes

- **Global**: `~/.buckets_config.toml`
- **Local**: `.buckets/config` in the current repository
- **Effective**: Local overrides global

## Key format

Keys use dot notation to address TOML tables:

- `network.ntp_server`
- `database.postgresql_connection`

## Examples

```bash
# Set global NTP server
buckets config set network.ntp_server time.google.com --global

# Set repository-specific database connection
buckets config set database.postgresql_connection postgresql://user:pass@localhost:5432/buckets --local

# Read effective value (local overrides global)
buckets config get network.ntp_server

# List effective configuration (merged)
buckets config list --effective

# Remove a value from global config
buckets config unset database.postgresql_connection --global
```

## Suggested categories

Global:
- `[core]` default_repo_path, default_editor, color, json
- `[network]` ntp_server, ip_check, url_check, timeout_sec
- `[database]` postgresql_connection, pool_max_size, pool_timeout_sec
- `[paths]` cache_dir, temp_dir

Local:
- `[core]` repo_name, default_bucket
- `[database]` postgresql_connection, database, schema
- `[storage]` compression_level, hash_algorithm, storage_path
- `[workflow]` require_expectations, auto_link
