# Setup Command Documentation

## Overview

The `setup` command provides global configuration management for Buckets CLI. It allows users to configure system-wide settings that are inherited by all Buckets repositories on the local machine.

## Purpose

- **Global Configuration**: Set up system-wide defaults for all Buckets repositories
- **Database Configuration**: Configure PostgreSQL connection strings for external databases
- **NTP Configuration**: Set custom NTP servers for time synchronization
- **User Experience**: Streamline repository setup with pre-configured defaults

## Syntax

```bash
buckets setup [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Enable verbose output |
| `--json` | Output configuration in JSON format |
| `--test-connection` | Test the database connection after configuration |
| `-h, --help` | Display help information |

## Usage

### Basic Usage

```bash
# Interactive configuration setup
buckets setup
```

The command will guide you through an interactive configuration process, asking for:

1. **PostgreSQL Connection String** (optional)
2. **NTP Server** (defaults to "pool.ntp.org")

### Example Interactive Session

```bash
$ buckets setup
Buckets Global Configuration Setup
=================================

PostgreSQL Configuration
------------------------
No PostgreSQL connection string configured
Enter PostgreSQL connection string (or press Enter to keep current): 

NTP Server Configuration
------------------------
Current NTP server: pool.ntp.org
Enter NTP server (or press Enter to keep current): time.google.com
NTP server updated


Global configuration saved successfully!
Configuration file: /home/user/.buckets_config.toml
```

## Configuration Options

### PostgreSQL Connection String

Configure an external PostgreSQL database connection that will be used by default for new repositories.

**Format:**
```
postgresql://username:password@host:port/database
```

**Examples:**
```bash
# Local PostgreSQL instance
postgresql://buckets_user:password@localhost:5432/buckets_db

# Remote PostgreSQL server
postgresql://user:pass@db.example.com:5432/production_buckets

# PostgreSQL with SSL
postgresql://user:pass@secure-db.com:5432/buckets?sslmode=require
```

**When to use:**
- Shared team database setups
- Production environments with dedicated database servers
- When you need consistent database configuration across multiple repositories

### Database Connection Testing

Test your PostgreSQL connection configuration to ensure it's working correctly.

```bash
# Test connection after setup
buckets setup --test-connection

# Setup with connection testing
buckets setup --test-connection
```

**What it tests:**
- Connection pool creation
- Database server connectivity  
- Authentication credentials
- Basic query execution (`SELECT 1`)

**Example output:**
```bash
Testing Database Connection
===========================
Testing PostgreSQL connection: postgresql://user:***@localhost:5432/buckets
✅ PostgreSQL connection successful!
```

**Connection testing features:**
- **Password masking**: Passwords are hidden in output for security
- **Timeout handling**: 10-second connection timeout to prevent hanging
- **Error reporting**: Clear error messages for connection failures
- **Authentication validation**: Verifies username/password combination
- **Query testing**: Ensures database is accessible and responsive

**Common connection issues:**
- **Connection refused**: Database server not running or wrong port
- **Authentication failed**: Incorrect username/password
- **Database not found**: Specified database doesn't exist
- **Network timeout**: Server unreachable or network issues
- **SSL errors**: SSL configuration mismatch

### NTP Server Configuration

Set a custom Network Time Protocol (NTP) server for time synchronization.

**Default:** `pool.ntp.org`

**Common alternatives:**
```bash
time.google.com          # Google's public NTP
time.cloudflare.com      # Cloudflare's NTP service
pool.ntp.org            # Default NTP pool
time.nist.gov           # NIST time servers
0.pool.ntp.org          # Specific pool server
```

**When to customize:**
- Corporate environments with internal NTP servers
- Regions with better local NTP server performance
- High-precision time requirements

## Configuration File

The setup command creates and manages a configuration file at:

**Location:** `~/.buckets_config.toml`

### File Format

```toml
[network]
ntp_server = "time.google.com"

[database]
postgresql_connection = "postgresql://user:pass@localhost:5432/buckets"
```

### File Structure

| Field | Type | Description | Required |
|-------|------|-------------|----------|
| `network.ntp_server` | String | NTP server hostname or IP | Yes |
| `database.postgresql_connection` | String | PostgreSQL connection string | No |

## Integration with Repositories

### Configuration Inheritance

When you initialize a new repository with `buckets init`, the global configuration is automatically inherited:

1. **NTP Server**: Repository config inherits the global NTP server setting
2. **Database Defaults**: PostgreSQL connection string is available for external database setups
3. **Override Capability**: Local repository settings can override global configuration

### Example Workflow

```bash
# 1. Set up global configuration
buckets setup
# Enter: postgresql://team:secret@db.company.com:5432/buckets
# Enter: time.company.com

# 2. Initialize new repository
buckets init my_project

# 3. Repository automatically inherits global settings
cd my_project
cat .buckets/config
# Will show network.ntp_server = "time.company.com"
```

## Command Behavior

### First Run

- Creates `~/.buckets_config.toml` with default values
- Prompts for PostgreSQL connection (optional)
- Prompts for NTP server (defaults to pool.ntp.org)

### Subsequent Runs

- Displays current configuration values
- Allows updating individual settings
- Preserves existing configuration if no changes made

### Error Handling

The setup command gracefully handles various error conditions:

**Missing Home Directory:**
```bash
Error: Could not find home directory
```

**Permission Issues:**
```bash
Error: Permission denied writing to ~/.buckets_config.toml
```

**Corrupted Configuration:**
```bash
Warning: Corrupted configuration file found, recreating...
```

**Interrupted Input:**
```bash
Configuration cancelled by user
```

## Use Cases

### 1. Development Environment Setup

```bash
# Set up development defaults
buckets setup
# Enter: postgresql://dev:dev@localhost:5432/buckets_dev
# Enter: pool.ntp.org

# All new repositories use development database
buckets init project1
buckets init project2
# Both inherit the development database configuration
```

### 2. Team Environment Configuration

```bash
# Configure for shared team database
buckets setup
# Enter: postgresql://team:teampass@shared-db:5432/team_buckets
# Enter: time.company.com

# Team members get consistent configuration
```

### 3. Production Environment

```bash
# Production server setup
buckets setup
# Enter: postgresql://prod_user:secure_pass@prod-db:5432/buckets_prod
# Enter: time.nist.gov

# Production repositories use production database
```

### 4. Offline/Local Development

```bash
# Configure for local-only development
buckets setup
# Enter: (leave empty if configuring per-repository or via env vars)
```

## Security Considerations

### Configuration File Security

The configuration file may contain sensitive information:

```bash
# Set appropriate file permissions
chmod 600 ~/.buckets_config.toml
```

### Database Connection Strings

**⚠️ Security Warning:** The PostgreSQL connection string is stored in plain text.

**Best Practices:**
- Use environment variables for sensitive credentials
- Set restrictive file permissions (600)
- Avoid storing production passwords in the configuration file
- Consider using PostgreSQL connection service files

**Alternative Secure Approach:**
```bash
# Instead of storing password in config, use .pgpass file
# or environment variables
export PGPASSWORD=secure_password
```

## Troubleshooting

### Common Issues

**Problem:** Configuration file not found
```bash
# Solution: Run setup to create configuration
buckets setup
```

**Problem:** Repository not inheriting global config
```bash
# Check configuration exists
cat ~/.buckets_config.toml

# Re-initialize repository if needed
buckets init --force repo_name
```

**Problem:** PostgreSQL connection fails
```bash
# Test connection manually
psql postgresql://user:pass@host:port/database

# Update configuration
buckets setup
```

**Problem:** Permission denied on configuration file
```bash
# Fix permissions
chmod 644 ~/.buckets_config.toml

# Or recreate configuration
rm ~/.buckets_config.toml
buckets setup
```

### Debugging

**Enable verbose mode:**
```bash
buckets setup --verbose
```

**Check configuration file manually:**
```bash
cat ~/.buckets_config.toml
```

**Validate TOML syntax:**
```bash
# Using external tools if available
toml-lint ~/.buckets_config.toml
```

## Environment Variables

The setup command respects certain environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `HOME` | User home directory for config file | `/home/username` |
| `DATABASE_URL` | Default PostgreSQL URL (displayed) | `postgresql://...` |

## Examples

### Complete Configuration Examples

**Minimal Configuration:**
```toml
[network]
ntp_server = "pool.ntp.org"
```

**Full Configuration:**
```toml
[network]
ntp_server = "time.google.com"

[database]
postgresql_connection = "postgresql://buckets:secret@db.example.com:5432/buckets_prod"
```

**Corporate Environment:**
```toml
[network]
ntp_server = "ntp.company.internal"

[database]
postgresql_connection = "postgresql://app_user:app_pass@db.company.internal:5432/buckets"
```

## Integration with Other Commands

### Commands Affected by Global Configuration

1. **`buckets init`** - Inherits NTP server setting
2. **Repository operations** - Use inherited database settings
3. **Time-sensitive operations** - Use configured NTP server

### Commands Not Affected

Global configuration does not affect:
- Existing repository configurations
- Command-line overrides
- Local repository-specific settings

## Migration and Updates

### Updating Configuration

```bash
# Update configuration at any time
buckets setup

# Only change what you need
# Press Enter to keep existing values
```

### Migrating Between Environments

```bash
# Export current configuration
cp ~/.buckets_config.toml ~/.buckets_config.backup

# Set up new environment
buckets setup
# Configure for new environment

# Restore if needed
cp ~/.buckets_config.backup ~/.buckets_config.toml
```

### Version Compatibility

The configuration file format is forward-compatible:
- New fields can be added without breaking existing configurations
- Unknown fields are ignored by older versions
- Required fields maintain backward compatibility

## Best Practices

### 1. Environment-Specific Configuration

```bash
# Development
buckets setup
# Use: postgresql://dev:dev@localhost:5432/buckets_dev

# Staging  
buckets setup
# Use: postgresql://staging:staging@staging-db:5432/buckets_staging

# Production
buckets setup
# Use: postgresql://prod:prod@prod-db:5432/buckets_prod
```

### 2. Team Consistency

- Document team configuration settings
- Use shared NTP servers for time consistency
- Standardize database connection patterns
- Version control setup scripts (without passwords)

### 3. Security

```bash
# Set restrictive permissions
chmod 600 ~/.buckets_config.toml

# Use .pgpass for password management
echo "hostname:port:database:username:password" > ~/.pgpass
chmod 600 ~/.pgpass
```

### 4. Documentation

Document your setup configuration:

```markdown
# Team Setup Configuration

## Database
- Host: db.team.local
- Port: 5432
- Database: team_buckets
- User: team_user

## NTP
- Server: time.team.local

## Setup Command
```bash
buckets setup
# Enter: postgresql://team_user:TEAM_PASSWORD@db.team.local:5432/team_buckets
# Enter: time.team.local
```
```

## Related Commands

- [`buckets init`](init_command.md) - Initialize repository (inherits global config)
- [`buckets list`](list_command.md) - List repositories
- [`buckets --help`](help.md) - General help

## See Also

- [PostgreSQL Connection Strings](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING)
- [NTP Pool Project](https://www.pool.ntp.org/)
- [TOML Specification](https://toml.io/)
