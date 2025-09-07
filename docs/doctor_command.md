# Doctor Command Documentation

## Overview

The `doctor` command provides comprehensive system diagnostics for Buckets CLI. It performs health checks on critical system components including database connectivity and NTP server reachability to help users diagnose configuration issues.

## Purpose

- **System Health Checks**: Validate that all configured services are operational
- **Database Connectivity**: Test PostgreSQL connections (both global and repository configurations)
- **NTP Server Testing**: Verify time server connectivity and responsiveness
- **Configuration Validation**: Ensure both global and repository configurations are working
- **Troubleshooting**: Provide clear diagnostics for common system issues

## Syntax

```bash
buckets doctor [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Enable verbose output with detailed error messages |
| `--json` | Output results in JSON format for automation |
| `--skip-database` | Skip database connection test |
| `--skip-ntp` | Skip NTP server test |
| `--use-repo` | Use repository config instead of global config |
| `-h, --help` | Display help information |

## Usage

### Basic Usage

```bash
# Run all diagnostics (recommended)
buckets doctor

# Run with verbose error reporting
buckets doctor --verbose

# JSON output for automation
buckets doctor --json
```

### Selective Testing

```bash
# Test only database connection
buckets doctor --skip-ntp

# Test only NTP server
buckets doctor --skip-database

# Use repository configuration instead of global
buckets doctor --use-repo
```

## Example Output

### Text Format (Default)

```bash
$ buckets doctor
Buckets System Diagnostics
==========================

Database Connection Test
------------------------
Using global configuration
Testing connection: postgresql://user:***@localhost:5432/buckets
✅ Database connection successful
   Connection time: 45ms

NTP Server Test
---------------
Testing NTP server: pool.ntp.org
✅ NTP server reachable
   Response time: 23ms
   NTP query successful

Summary
-------
✅ All systems operational
```

### JSON Format

```json
{
  "timestamp": "2025-09-07T12:00:00.000000000+00:00",
  "tests": {
    "database": {
      "status": "passed",
      "connection_string": "postgresql://user:***@localhost:5432/buckets",
      "config_source": "global",
      "connection_time_ms": 45,
      "test_timestamp": "2025-09-07T12:00:01.000000000+00:00"
    },
    "ntp": {
      "status": "passed",
      "server": "pool.ntp.org",
      "config_source": "global",
      "response_time_ms": 23,
      "test_timestamp": "2025-09-07T12:00:02.000000000+00:00"
    }
  },
  "summary": {
    "status": "passed",
    "all_passed": true
  }
}
```

### Error Output

```bash
$ buckets doctor
Buckets System Diagnostics
==========================

Database Connection Test
------------------------
Using global configuration
Testing connection: postgresql://user:***@localhost:5432/baddb
❌ Database connection failed
   Error: Failed to connect to PostgreSQL database: connection refused

NTP Server Test
---------------
Testing NTP server: invalid.ntp.server
❌ NTP server unreachable
   Error: Failed to resolve NTP server 'invalid.ntp.server': name resolution failed

Summary
-------
❌ Some issues detected
```

## Test Details

### Database Connection Test

**What it tests:**
- Connection pool creation and configuration
- PostgreSQL server connectivity
- Authentication with provided credentials
- Basic query execution (`SELECT 1`)
- Connection response time measurement

**Configuration sources:**
- **Global config** (default): Uses `~/.buckets_config.toml`
- **Repository config** (with `--use-repo`): Uses `.buckets/config` in current repository

**Success criteria:**
- TCP connection established within 10-second timeout
- Authentication successful with provided credentials
- Test query executes without errors
- Connection pool created successfully

**Common failure scenarios:**
- **Connection refused**: Database server not running or wrong port
- **Authentication failed**: Incorrect username/password combination
- **Database not found**: Specified database doesn't exist on the server
- **Network timeout**: Server unreachable due to network issues
- **SSL errors**: SSL/TLS configuration mismatch

### NTP Server Test

**What it tests:**
- DNS resolution of NTP server hostname
- UDP connectivity to port 123
- NTP protocol query and response
- Server response time measurement

**Configuration sources:**
- **Global config** (default): Uses `ntp_server` from `~/.buckets_config.toml`
- **Repository config** (with `--use-repo`): Uses `ntp_server` from `.buckets/config`
- **Fallback**: Uses `pool.ntp.org` if no configuration found

**Success criteria:**
- NTP server hostname resolves to valid IP address
- UDP connection established to port 123
- Valid NTP response received
- Response time measured successfully

**Common failure scenarios:**
- **Name resolution failed**: DNS cannot resolve the NTP server hostname
- **Connection timeout**: NTP server not responding or blocked by firewall
- **Invalid response**: Server responds but with invalid NTP data
- **Network unreachable**: Routing issues preventing UDP traffic

## Configuration Integration

The doctor command intelligently selects configuration sources:

### Global Configuration Priority
```bash
# Uses ~/.buckets_config.toml by default
buckets doctor

# Equivalent to:
# - Database: global PostgreSQL connection string
# - NTP: global NTP server setting
```

### Repository Configuration Priority
```bash
# Uses .buckets/config in current directory
buckets doctor --use-repo

# Equivalent to:
# - Database: repository PostgreSQL connection (if configured)
# - NTP: repository NTP server setting
```

### Configuration Fallbacks
- **No global config**: Uses hardcoded defaults where possible
- **No repository config**: Returns error with `--use-repo`
- **Partial config**: Tests available components, skips missing ones

## Security Features

### Password Masking
All database connection strings are automatically masked in output:
- **Original**: `postgresql://user:secretpassword@host:5432/db`
- **Displayed**: `postgresql://user:***@host:5432/db`

### Safe Error Reporting
- Connection errors don't expose sensitive information
- Timeout handling prevents hanging operations
- Clear error categories without revealing system internals

## Automation and Integration

### Exit Codes
- **0**: All tests passed successfully
- **1**: One or more tests failed

### JSON Output Schema
```json
{
  "timestamp": "ISO8601 timestamp",
  "tests": {
    "database": {
      "status": "passed|failed",
      "connection_string": "masked connection string",
      "config_source": "global|repository",
      "connection_time_ms": 45,
      "test_timestamp": "ISO8601 timestamp",
      "error": "error message (if failed)"
    },
    "ntp": {
      "status": "passed|failed", 
      "server": "ntp.server.hostname",
      "config_source": "global|repository",
      "response_time_ms": 23,
      "test_timestamp": "ISO8601 timestamp",
      "error": "error message (if failed)"
    }
  },
  "summary": {
    "status": "passed|failed",
    "all_passed": true
  }
}
```

### Monitoring Integration
```bash
# Check if all systems operational (exit code 0)
if buckets doctor --json > /dev/null 2>&1; then
    echo "All systems OK"
else
    echo "System issues detected"
    buckets doctor --json | jq '.tests'
fi
```

## Troubleshooting

### Database Issues

**"No global PostgreSQL configuration found"**
- Run `buckets setup` to configure global database connection
- Ensure `~/.buckets_config.toml` exists and contains `postgresql_connection`

**"Failed to connect to PostgreSQL database"**
- Verify PostgreSQL server is running
- Check connection parameters (host, port, database name)
- Validate credentials (username, password)
- Test network connectivity to database server

**"Connection timeout"**
- Check if PostgreSQL server is accepting connections
- Verify firewall settings allow connections to PostgreSQL port
- Ensure PostgreSQL `postgresql.conf` allows external connections

### NTP Issues

**"Failed to resolve NTP server"**
- Verify DNS resolution: `nslookup pool.ntp.org`
- Check network connectivity
- Try alternative NTP server: `time.google.com`

**"NTP server unreachable"**
- Verify UDP port 123 is not blocked by firewall
- Test with different NTP server
- Check if running in restricted network environment

**"Invalid NTP response"**
- Server may not be running NTP service
- Network interference or packet corruption
- Try well-known public NTP servers

### Configuration Issues

**"Cannot find repository configuration"**
- Ensure you're running command inside a Buckets repository
- Check if `.buckets/config` file exists
- Initialize repository with `buckets init` if needed

**"Not in a Buckets repository"**
- Navigate to a directory containing `.buckets/` subdirectory
- Remove `--use-repo` flag to use global configuration instead

## Best Practices

### Regular Health Checks
```bash
# Daily system check
buckets doctor

# Include in CI/CD pipelines
buckets doctor --json && echo "Systems healthy"
```

### Configuration Validation
```bash
# After setup changes
buckets setup
buckets doctor --verbose

# Before critical operations
buckets doctor && buckets commit "Important changes"
```

### Monitoring Integration
```bash
# JSON output for log aggregation
buckets doctor --json | logger -t buckets-health

# Automated alerting
buckets doctor || notify-send "Buckets system issues detected"
```

## Related Commands

- **`buckets setup`**: Configure global settings tested by doctor
- **`buckets setup --test-connection`**: Test database connection during setup
- **`buckets init`**: Initialize repository with configuration
- **`buckets status`**: Check repository state (different from system health)

## See Also

- [Setup Command Documentation](setup_command.md)
- [Configuration Management Guide](../CLAUDE.md#configuration)
- [Troubleshooting Guide](../README.md#troubleshooting)
- [Manual Testing Documentation](manual_testing.md)