# buckets init

Initialize a new bucket repository with PostgreSQL database backend.

## Synopsis

```bash
buckets init [OPTIONS] <REPO_NAME>
```

## Description

The `init` command creates a new bucket repository in the current directory. A bucket repository is a workspace for managing game assets and tracking expectations between collaborators. Each repository contains buckets (versioned asset containers) and uses an external PostgreSQL database to store metadata, commit history, and file relationships.

The command creates:
- A new directory named `<REPO_NAME>`
- A `.buckets` configuration directory inside the repository
- Configuration for connecting to an external PostgreSQL database
- Essential configuration files
- Database schema and initial setup

## Arguments

### `<REPO_NAME>`
**Required.** The name of the repository to create.

- Must be a valid directory name
- Cannot conflict with existing files or directories
- Will be created in the current working directory

## Options

### External Database Configuration
**Required.** Options for connecting to an external PostgreSQL database:

- `--external-host <HOST>` - PostgreSQL server hostname or IP address
- `--external-port <PORT>` - PostgreSQL server port (default: 5432)
- `--external-database <DB>` - Database name (default: buckets)
- `--external-username <USER>` - Database username
- `--external-password <PASS>` - Database password (optional)

### Global Options
See [Global Options](../global-options.md) for shared command options.

## External PostgreSQL Database

### Basic Setup
```bash
buckets init my-project --external-host localhost --external-username myuser --external-password mypass
```

### Full Configuration
```bash
buckets init my-project \
  --external-host db.example.com \
  --external-port 5432 \
  --external-database buckets \
  --external-username gamedev \
  --external-password secretpass
```

**Requirements:**
- External PostgreSQL server running and accessible
- Database user with CREATE privileges
- Network connectivity to the database server
- Approximately 50MB disk space for PostgreSQL installation
- Port availability (auto-selected if default unavailable)

### External PostgreSQL
```bash
buckets init my-project --database external
```

- Connect to existing PostgreSQL server
- Shared across multiple repositories if desired
- Centralized database management
- Better for team/production environments

## Environment Variables

### `DATABASE_URL` (Alternative Configuration)
Optionally, you can use the `DATABASE_URL` environment variable instead of command-line options:
```bash
DATABASE_URL="postgresql://username:password@host:port/database"
```

**Format:** `postgresql://[username[:password]@]host[:port]/database`

**Examples:**
```bash
# Local PostgreSQL with authentication
DATABASE_URL="postgresql://buckets_user:secret123@localhost:5432/buckets_db"

# Local PostgreSQL with certificate-authenticated role (no password in URL)
DATABASE_URL="postgresql://buckets_cli@db.example.com:5432/buckets_db"

# Remote PostgreSQL
DATABASE_URL="postgresql://user:pass@db.example.com:5432/my_buckets"

# When using DATABASE_URL, no --external-* options are needed
buckets init my-project
```

### mTLS configuration (optional)

`buckets init` reads TLS settings from global config (`~/.buckets_config.toml`) when present.
To enable mutual TLS, configure:

```toml
[database.tls]
ca_cert = "/etc/buckets/certs/root_ca.crt"
client_cert = "/etc/buckets/certs/buckets_cli.crt"
client_key = "/etc/buckets/certs/buckets_cli.key"
```

## Examples

### Basic Repository Creation
```bash
# Create repository with external PostgreSQL
buckets init my-game-project --external-host localhost --external-username myuser --external-password mypass
```

### Using Environment Variable
```bash
# Set up database connection
export DATABASE_URL="postgresql://buckets_user:password@localhost:5432/buckets_db"

# Create repository using DATABASE_URL
buckets init shared-project
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
│   └── db_config.toml           # External database configuration
└── (empty - ready for buckets)
```

### Configuration Files

#### `.buckets/config`
Repository configuration in TOML format:
```toml
[network]
ntp_server = "pool.ntp.org"
ip_check = "8.8.8.8"
url_check = "api.ipify.org"
```

#### `.buckets/db_config.toml`
External database configuration in TOML format:
```toml
type = "external"
host = "localhost"
port = 5432
database = "buckets"
username = "myuser"
password = "mypass"
```

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
- **4** - Database connection failed (external PostgreSQL)
- **5** - Database configuration invalid

## Error Handling

### Common Errors

#### Repository Already Exists
```
Error: Repository 'my-project' already exists
```
**Solution:** Choose a different name or remove the existing directory.

#### Database Connection Failed
```
Error: Failed to connect to database: connection refused
```
**Solution:** 
- Verify PostgreSQL server is running
- Check database connection parameters
- Ensure network connectivity to database server

#### Missing Database Configuration
```
Error: External database configuration is required. Please provide --external-host and --external-username options.
```
**Solution:** Provide the required external database connection parameters.

#### Permission Denied
```
Error: Permission denied (os error 13)
```
**Solution:** Ensure write permissions in the current directory.

### Database Connection Issues

#### PostgreSQL Not Accessible
```
Error: Failed to connect to database: connection refused
```
**Solutions:**
- Verify PostgreSQL server is running
- Check database connection parameters and credentials
- Ensure network connectivity to database server
- Verify firewall settings

## Integration

### With Version Control
```bash
# Initialize repository
buckets init my-project --external-host localhost --external-username myuser --external-password mypass
cd my-project

# Initialize git (recommended)
git init
git add .
git commit -m "Initial bucket repository setup"
```

### CI/CD Considerations
```bash
# For automated environments, use external database
export DATABASE_URL="postgresql://ci_user:$DB_PASSWORD@postgres-server:5432/ci_buckets"
buckets init build-artifacts
```

## Performance Notes

### External PostgreSQL
- **Initial setup:** Seconds (no download required)
- **Connection overhead:** Minimal network latency
- **Shared resources:** Multiple repositories can share one database
- **Scalability:** Professional database management and optimization

## Security Considerations

### External PostgreSQL
- Database credentials in configuration files or environment variables
- Network communication with PostgreSQL server
- Shared database security responsibilities

**Recommendations:**
- Use strong passwords for database connections
- Encrypt database connections (SSL/TLS)
- Regular database backups
- Restrict database user permissions appropriately
- Keep configuration files secure and out of version control

## Troubleshooting

### Database Connection Issues
```bash
# Test database connection
psql $DATABASE_URL -c "SELECT version();"

# Verify environment variable
echo $DATABASE_URL

# Check database permissions
psql $DATABASE_URL -c "SELECT current_user;"

# Test connection with explicit parameters
psql -h hostname -p 5432 -U username -d database -c "SELECT version();"
```

## Migration Notes

When upgrading from DuckDB-based buckets repositories, see [Migration Guide](../migration/duckdb-to-postgresql.md) for detailed migration instructions.

## See Also

- [buckets create](create.md) - Create a new bucket
- [buckets status](status.md) - Check repository status  
- [Global Options](../global-options.md) - Common command options
- [Migration Guide](../migration/duckdb-to-postgresql.md) - Upgrading from DuckDB
