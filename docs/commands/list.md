# buckets list

## SYNOPSIS
```
buckets list [OPTIONS]
```

## DESCRIPTION
Display all buckets in the current repository. The list command queries the PostgreSQL database to retrieve all buckets that have been created in the repository, showing their names, unique IDs, and relative paths.

This command provides an overview of the bucket structure in your repository, helping you navigate between buckets and understand the organization of your assets.

## ARGUMENTS
None - the command takes no positional arguments.

## OPTIONS
- `-v, --verbose` - Enable verbose output with debug information
- `--json` - Output results in JSON format for programmatic use
- `-h, --help` - Show help information

## OUTPUT FORMATS

### Default Format
```
Buckets:
  character_assets - 550e8400-e29b-41d4-a716-446655440000 (character_assets)
  level_design - 6ba7b810-9dad-11d1-80b4-00c04fd430c8 (levels/level_design)
  audio_files - 7ca8b910-8ead-22e2-90c5-11d05de530d9 (audio/audio_files)
```

### JSON Format (with --json flag)
```json
{
  "buckets": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "character_assets",
      "path": "character_assets"
    },
    {
      "id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "name": "level_design", 
      "path": "levels/level_design"
    },
    {
      "id": "7ca8b910-8ead-22e2-90c5-11d05de530d9",
      "name": "audio_files",
      "path": "audio/audio_files"
    }
  ]
}
```

### Empty Repository
```
No buckets found
```

## BEHAVIOR

### Repository Validation
- Verifies the current directory is a valid buckets repository
- Checks for the presence of `.buckets/` metadata directory
- Returns error if not in a repository

### Database Query
The command executes a SQL query to retrieve buckets:
```sql
SELECT id, name, path FROM buckets
```

### Async Operations
- Uses Tokio async runtime for database operations
- Non-blocking database queries for better performance
- Handles database connection pooling automatically

### Output Formatting
- **Default**: Human-readable table format with bucket names, IDs, and paths
- **JSON**: Structured data suitable for scripts and tools
- **Empty**: Clear message when no buckets exist

## EXAMPLES

### Basic Bucket Listing
```bash
# Show all buckets in the repository
buckets list
```

### JSON Output for Scripts
```bash
# Get bucket information in JSON format
buckets list --json

# Extract bucket names for processing
buckets list --json | jq -r '.buckets[].name'

# Count total buckets
buckets list --json | jq '.buckets | length'
```

### Verbose Output
```bash
# Show detailed debug information during listing
buckets list -v
```

### Integration with Other Commands
```bash
# List buckets and navigate to one
buckets list
cd character_assets

# Create a new bucket and verify it appears
buckets create new_bucket
buckets list
```

## REPOSITORY CONTEXT
The list command must be run from within a buckets repository (a directory containing `.buckets/` metadata). It will only show buckets that belong to the current repository.

## DATABASE SCHEMA
The command queries the PostgreSQL buckets table:

### buckets table
```sql
CREATE TABLE buckets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    path TEXT NOT NULL
);
```

**Field Descriptions:**
- `id` - Unique identifier for the bucket (UUID v4)
- `name` - Human-readable bucket name
- `path` - Relative path from repository root to bucket directory

## EXIT STATUS
- `0` - Success: Buckets retrieved and displayed successfully
- `1` - Error: Not in repository, database connection failed, or other error

## ENVIRONMENT VARIABLES
- `BUCKETS_LOG_LEVEL` - Set logging level (error, warn, info, debug, trace)
- `RUST_LOG` - Standard Rust logging configuration (overrides BUCKETS_LOG_LEVEL)
- `POSTGRES_HOST` - PostgreSQL host (for external database mode)
- `POSTGRES_PORT` - PostgreSQL port (for external database mode)
- `POSTGRES_USER` - PostgreSQL username (for external database mode)
- `POSTGRES_PASSWORD` - PostgreSQL password (for external database mode)
- `POSTGRES_DB` - PostgreSQL database name (for external database mode)

## ERROR HANDLING

### Common Error Scenarios

#### Not in Repository
```
Error: Not currently in a buckets repository
```
**Solution**: Navigate to a directory initialized with `buckets init` or containing `.buckets/` directory.

#### Database Connection Failed
```
Error: Failed to connect to PostgreSQL database
```
**Solution**: Ensure PostgreSQL is running or check connection settings.

#### Invalid Database Data
```
Error: Invalid data: Invalid UUID format
```
**Solution**: This indicates database corruption; consider repository repair or recovery from backup.

#### JSON Serialization Error
```
Error serializing to JSON: <error details>
```
**Solution**: This indicates a data consistency issue; check database integrity.

### Troubleshooting

1. **Empty Bucket List**
   - Verify you're in the correct repository
   - Check if buckets were created with `buckets create`
   - Ensure database is accessible and contains bucket data

2. **Performance Issues**
   - Large numbers of buckets may slow query performance
   - Consider database indexing for improved performance
   - Monitor PostgreSQL query execution time

3. **Path Display Issues**
   - Relative paths are stored as-is from creation
   - Moved directories may show stale paths
   - Use bucket IDs for reliable identification

## PERFORMANCE CONSIDERATIONS

### Query Performance
- Simple SELECT query without joins
- Performance scales linearly with number of buckets
- PostgreSQL indexes on primary key provide fast access

### Memory Usage
- All bucket records are loaded into memory
- Minimal memory footprint for typical repositories
- Large repositories with thousands of buckets may use more RAM

### Network Performance (External PostgreSQL)
- Query results are transferred over network
- JSON format has minimal overhead
- Connection pooling reduces overhead for multiple commands

## INTEGRATION WITH OTHER COMMANDS

### Workflow Integration
```bash
# List available buckets
buckets list

# Navigate to specific bucket
cd character_assets

# Work with bucket contents
buckets status
buckets commit "Updated character models"

# Return to repository root and list again
cd ..
buckets list
```

### Scripting Integration
```bash
# Process all buckets in a script
for bucket in $(buckets list --json | jq -r '.buckets[].name'); do
  echo "Processing bucket: $bucket"
  cd "$bucket"
  buckets status
  cd ..
done
```

### Repository Management
```bash
# Verify repository structure
buckets list
ls -la  # Compare with filesystem

# Clean up unused buckets (manual)
buckets list --json | jq '.buckets[] | select(.name == "old_bucket")'
```

## SECURITY CONSIDERATIONS

### Data Exposure
- Bucket names and paths are visible to anyone who can run the command
- Sensitive bucket names may reveal project structure
- Consider access controls at the repository level

### Database Access
- Command requires read access to PostgreSQL database
- Connection credentials should be stored securely
- Use principle of least privilege for database users

## PLANNED ENHANCEMENTS

### Filtering Options (Future)
- `--name PATTERN` - Filter buckets by name pattern
- `--path PATTERN` - Filter buckets by path pattern
- `--created-after DATE` - Show recently created buckets
- `--sort FIELD` - Sort by name, path, or creation date

### Display Options (Future)
- `--format FORMAT` - Custom output formatting
- `--tree` - Tree view of bucket hierarchy
- `--stats` - Include commit counts and sizes
- `--long` - Detailed view with creation dates and commit info

### Performance (Future)
- Pagination support for large repositories
- Caching for frequently accessed bucket lists
- Incremental updates for better responsiveness

## MIGRATION NOTES

### From DuckDB (v0.1.x)
The list system was migrated from DuckDB to PostgreSQL in v0.2.0:
- Database schema adapted for PostgreSQL
- Query performance improved with native PostgreSQL types
- UUID handling enhanced with native PostgreSQL UUID functions
- All existing bucket data preserved during migration

### Async Runtime
The list command uses Tokio async runtime:
- Database operations are asynchronous for better performance
- Non-blocking I/O reduces command latency
- Error handling maintains full stack traces

### Thread-Local Storage
The command uses thread-local storage for current directory tracking:
- Improves performance by avoiding repeated directory lookups
- Ensures consistent behavior across command execution
- Handles directory changes gracefully

## SEE ALSO
- `buckets-init(1)` - Initialize a new bucket repository
- `buckets-create(1)` - Create new buckets
- `buckets-status(1)` - Show status of current bucket
- `buckets-history(1)` - View commit history across buckets
- `buckets-stats(1)` - Show detailed repository statistics