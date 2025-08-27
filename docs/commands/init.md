# buckets init

Initialize a new bucket repository with PostgreSQL database backend.

## Synopsis

```bash
buckets init [OPTIONS] <REPO_NAME>
```

## Description

The `init` command creates a new bucket repository in the current directory. A bucket repository is a workspace for managing game assets and tracking expectations between collaborators. Each repository contains buckets (versioned asset containers) and uses a PostgreSQL database to store metadata, commit history, and file relationships.

The command creates:
- A new directory named `<REPO_NAME>`
- A `.buckets` configuration directory inside the repository
- A PostgreSQL database (embedded by default)
- Essential configuration files
- Database schema and initial setup

## Arguments

### `<REPO_NAME>`
**Required.** The name of the repository to create.

- Must be a valid directory name
- Cannot conflict with existing files or directories
- Will be created in the current working directory

## Options

### `--database <TYPE>`
**Optional.** Database backend type to use. Default: `embedded`

**Valid values:**
- `embedded` - Embedded PostgreSQL server (recommended)
- `external` - Connect to external PostgreSQL server
- `postgresql` - Alias for `external`
- `postgres` - Alias for `external`

### Global Options
See [Global Options](../global-options.md) for shared command options.

## Database Types

### Embedded PostgreSQL (Default)
```bash
buckets init my-project --database embedded
```

- **Recommended for most users**
- Automatically downloads and manages PostgreSQL binaries
- No external setup required
- Data stored in `.buckets/postgres/` directory
- Isolated per repository

**Requirements:**
- Internet connection for initial setup (downloads PostgreSQL binaries)
- Approximately 50MB disk space for PostgreSQL installation
- Port availability (auto-selected if default unavailable)

### External PostgreSQL
```bash
buckets init my-project --database external
```

- Connect to existing PostgreSQL server
- Requires manual database setup
- Shared across multiple repositories if desired

**Requirements:**
- PostgreSQL 13+ server running and accessible
- Database connection details via environment variables
- Appropriate database permissions

## Environment Variables

When using external PostgreSQL (`--database external`):

### `DATABASE_URL`
Complete PostgreSQL connection string.
```bash
DATABASE_URL="postgresql://username:password@host:port/database"
```

**Format:** `postgresql://[username[:password]@]host[:port]/database`

**Examples:**
```bash
# Local PostgreSQL with authentication
DATABASE_URL="postgresql://buckets_user:secret123@localhost:5432/buckets_db"

# Remote PostgreSQL
DATABASE_URL="postgresql://user:pass@db.example.com:5432/my_buckets"

# Local PostgreSQL, no password
DATABASE_URL="postgresql://postgres@localhost:5432/buckets"
```

## Examples

### Basic Repository Creation
```bash
# Create repository with embedded PostgreSQL (recommended)
buckets init my-game-project

# Equivalent explicit syntax
buckets init my-game-project --database embedded
```

### External PostgreSQL Setup
```bash
# Set up database connection
export DATABASE_URL="postgresql://buckets_user:password@localhost:5432/buckets_db"

# Create repository using external database
buckets init shared-project --database external
```

### Corporate/Team Environment
```bash
# Use shared PostgreSQL server for team collaboration
export DATABASE_URL="postgresql://team_user:secure_pass@db.company.com:5432/game_assets"
buckets init team-project --database postgresql
```

## File Structure

After successful initialization, the repository structure is:

```
my-project/
├── .buckets/                    # Configuration directory
│   ├── config                   # Repository configuration (TOML)
│   ├── database_type            # Database type identifier
│   └── postgres/                # Embedded PostgreSQL data (if using embedded)
│       ├── data/                # PostgreSQL data directory
│       ├── install/             # PostgreSQL binaries
│       └── .pgpass              # PostgreSQL password file
└── (empty - ready for buckets)
```

### Configuration Files

#### `.buckets/config`
Repository configuration in TOML format:
```toml
ntp_server = "pool.ntp.org"
ip_check = "8.8.8.8"
url_check = "api.ipify.org"
```

#### `.buckets/database_type`
Contains the database type (`embedded`, `external`, etc.) for the repository.

## Database Schema

The PostgreSQL database includes these tables:

- **`buckets`** - Bucket metadata and configuration
- **`commits`** - Commit history and messages
- **`files`** - File versions and hashes
- **Database migrations** - Schema version management

Schema is automatically created and managed through database migrations.

## Exit Codes

- **0** - Success
- **1** - General error (invalid arguments, system error)
- **2** - Repository already exists
- **3** - Permission denied
- **4** - Network error (embedded PostgreSQL setup)
- **5** - Database connection failed (external PostgreSQL)

## Error Handling

### Common Errors

#### Repository Already Exists
```
Error: Repository 'my-project' already exists
```
**Solution:** Choose a different name or remove the existing directory.

#### Network Connection Failed
```
Error: Failed to install PostgreSQL: HTTP status client error (403 rate limit exceeded)
```
**Solution:** 
- Wait and retry (GitHub API rate limiting)
- Use external PostgreSQL instead
- Check internet connectivity

#### Invalid Database Type
```
Error: Invalid database type 'mysql'. Valid options are: embedded, external, postgresql
```
**Solution:** Use a valid database type from the supported list.

#### Permission Denied
```
Error: Permission denied (os error 13)
```
**Solution:** Ensure write permissions in the current directory.

### Database Connection Issues

#### External PostgreSQL Not Accessible
```
Error: Failed to connect to database: connection refused
```
**Solutions:**
- Verify PostgreSQL server is running
- Check DATABASE_URL format and credentials
- Ensure network connectivity to database server
- Verify firewall settings

## Integration

### With Version Control
```bash
# Initialize repository
buckets init my-project
cd my-project

# Initialize git (recommended)
git init
echo ".buckets/postgres/" >> .gitignore  # Exclude database files
git add .
git commit -m "Initial bucket repository setup"
```

### CI/CD Considerations
```bash
# For automated environments, use external database
export DATABASE_URL="postgresql://ci_user:$DB_PASSWORD@postgres-server:5432/ci_buckets"
buckets init build-artifacts --database external
```

## Performance Notes

### Embedded PostgreSQL
- **Initial setup:** 1-3 minutes (downloads ~50MB)
- **Subsequent operations:** Near-instant startup
- **Disk usage:** ~50MB + data size
- **Memory usage:** ~10-20MB baseline

### External PostgreSQL
- **Initial setup:** Seconds (no download required)
- **Connection overhead:** Minimal network latency
- **Shared resources:** Multiple repositories can share one database

## Security Considerations

### Embedded PostgreSQL
- Database files stored locally in `.buckets/postgres/`
- No network exposure by default
- Automatic password generation and management

### External PostgreSQL
- Database credentials in `DATABASE_URL` environment variable
- Network communication with PostgreSQL server
- Shared database security responsibilities

**Recommendations:**
- Use strong passwords for database connections
- Encrypt database connections (SSL/TLS) for external databases
- Regular database backups
- Restrict database user permissions appropriately

## Troubleshooting

### Embedded PostgreSQL Issues
```bash
# Check if PostgreSQL process is running
ps aux | grep postgres

# View PostgreSQL logs
cat .buckets/postgres/data/log/postgresql-*.log

# Force cleanup and retry
rm -rf .buckets/postgres/
buckets init my-project
```

### External PostgreSQL Issues
```bash
# Test database connection
psql $DATABASE_URL -c "SELECT version();"

# Verify environment variable
echo $DATABASE_URL

# Check database permissions
psql $DATABASE_URL -c "SELECT current_user;"
```

## Migration Notes

When upgrading from DuckDB-based buckets repositories, see [Migration Guide](../migration/duckdb-to-postgresql.md) for detailed migration instructions.

## See Also

- [buckets create](create.md) - Create a new bucket
- [buckets status](status.md) - Check repository status  
- [Global Options](../global-options.md) - Common command options
- [Migration Guide](../migration/duckdb-to-postgresql.md) - Upgrading from DuckDB