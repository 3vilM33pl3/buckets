# buckets history

## SYNOPSIS
```
buckets history [OPTIONS]
```

## DESCRIPTION
Display the commit history for all buckets in the current repository. The history command shows a chronological list of all commits across all buckets, providing commit IDs, messages, timestamps, and bucket names.

This command queries the PostgreSQL database to retrieve commit records and displays them in reverse chronological order (newest first). It's useful for tracking changes, finding specific commits for reversion, and understanding project evolution.

## ARGUMENTS
None - the command takes no positional arguments.

## OPTIONS
- `-v, --verbose` - Enable verbose output (currently unused, reserved for future enhancements)
- `--json` - Output results in JSON format for programmatic use
- `-h, --help` - Show help information

## OUTPUT FORMATS

### Default Format
```
Commit History:
----------------------------------------
Commit ID: 550e8400-e29b-41d4-a716-446655440000
Message: Add player character models and textures
Created At: 2024-01-15 14:30:25.123456 UTC
Bucket: character_assets
----------------------------------------
Commit ID: 6ba7b810-9dad-11d1-80b4-00c04fd430c8
Message: Initial level geometry
Created At: 2024-01-15 10:15:00.654321 UTC
Bucket: level_design
----------------------------------------
```

### JSON Format (with --json flag)
```json
{
  "commits": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "message": "Add player character models and textures",
      "created_at": "2024-01-15 14:30:25.123456 UTC",
      "bucket_name": "character_assets"
    },
    {
      "id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "message": "Initial level geometry",
      "created_at": "2024-01-15 10:15:00.654321 UTC",
      "bucket_name": "level_design"
    }
  ]
}
```

## BEHAVIOR

### Database Query
The command executes a SQL query to retrieve commits:
```sql
SELECT c.id, c.message, c.created_at::text, b.name as bucket_name 
FROM commits c 
JOIN buckets b ON c.bucket_id = b.id 
ORDER BY c.created_at DESC
```

### Async Operations
- Uses Tokio async runtime for database operations
- Non-blocking database queries for better performance
- Handles database connection pooling automatically

### Ordering
- Commits are ordered by creation timestamp (newest first)
- All buckets in the repository are included
- No filtering by bucket or date range (planned for future versions)

## EXAMPLES

### Basic History Display
```bash
# Show commit history for all buckets
buckets history
```

### JSON Output for Scripts
```bash
# Get history in JSON format for processing
buckets history --json | jq '.commits[0].message'
```

### Finding Recent Commits
```bash
# Show recent commits and pipe to head for latest 5
buckets history | head -25  # 5 commits × 5 lines each
```

## REPOSITORY CONTEXT
The history command must be run from within a buckets repository (a directory containing `.buckets/` metadata). It will show commits from all buckets in the current repository.

## DATABASE SCHEMA
The command queries these PostgreSQL tables:

### commits table
```sql
CREATE TABLE commits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bucket_id UUID NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### buckets table
```sql
CREATE TABLE buckets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    relative_bucket_path TEXT NOT NULL
);
```

## EXIT STATUS
- `0` - Success: History retrieved and displayed successfully
- `1` - Error: Database connection failed, not in repository, or other error

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

#### No Commits Found
If no commits exist, the command displays:
```
Commit History:
----------------------------------------
```
**Note**: This is normal behavior for newly created repositories with no commits.

#### JSON Serialization Error
```
Error serializing to JSON: <error details>
```
**Solution**: This indicates a data consistency issue; check database integrity.

### Troubleshooting

1. **Empty History**
   - Verify commits exist with `buckets status` in bucket directories
   - Check if you're in the correct repository
   - Ensure commits were successfully created

2. **Performance Issues**
   - Large repositories may take time to query all commits
   - Consider database indexing for improved performance
   - Monitor PostgreSQL query performance

3. **Timestamp Display Issues**
   - Timestamps are displayed in UTC
   - Format may vary based on PostgreSQL configuration
   - Use JSON output for programmatic timestamp parsing

## PERFORMANCE CONSIDERATIONS

### Query Performance
- The command joins commits and buckets tables
- Performance scales with total number of commits
- PostgreSQL indexes on `bucket_id` and `created_at` improve performance

### Memory Usage
- All commit records are loaded into memory
- Large histories may consume significant RAM
- Consider pagination for repositories with thousands of commits

### Network Performance (External PostgreSQL)
- Query results are transferred over network
- JSON format may be more efficient for programmatic use
- Connection pooling reduces overhead for multiple queries

## INTEGRATION WITH OTHER COMMANDS

### Workflow Integration
```bash
# View history to find commits
buckets history

# Copy commit ID and revert
buckets revert 550e8400-e29b-41d4-a716-446655440000

# Check status after revert
buckets status
```

### Scripting Integration
```bash
# Get latest commit ID for automation
LATEST_COMMIT=$(buckets history --json | jq -r '.commits[0].id')
echo "Latest commit: $LATEST_COMMIT"
```

### Bucket-Specific History
Currently, the command shows all buckets. For bucket-specific history:
```bash
# Filter history by bucket name (using shell tools)
buckets history --json | jq '.commits[] | select(.bucket_name == "my_bucket")'
```

## PLANNED ENHANCEMENTS

### Filtering Options (Future)
- `--bucket NAME` - Show history for specific bucket only
- `--since DATE` - Show commits since specific date
- `--until DATE` - Show commits until specific date
- `--author USER` - Filter by commit author (when user tracking is added)

### Display Options (Future)
- `--limit N` - Limit number of commits displayed
- `--format FORMAT` - Custom output formatting
- `--graph` - ASCII graph of commit relationships
- `--stat` - Show file change statistics per commit

### Performance (Future)
- Pagination support for large histories
- Incremental loading for better responsiveness
- Caching for frequently accessed histories

## MIGRATION NOTES

### From DuckDB (v0.1.x)
The history system was migrated from DuckDB to PostgreSQL in v0.2.0:
- Query syntax updated for PostgreSQL compatibility
- Timestamp formatting improved with native PostgreSQL types
- Join performance significantly enhanced
- All existing commit data preserved during migration

### Async Runtime
The history command uses Tokio async runtime:
- Database operations are asynchronous for better performance
- Non-blocking I/O reduces latency
- Error handling maintains full stack traces

## SEE ALSO
- `buckets-init(1)` - Initialize a new bucket repository
- `buckets-commit(1)` - Create new commits
- `buckets-revert(1)` - Revert to previous commits
- `buckets-status(1)` - Show current status
- `buckets-stats(1)` - Show repository statistics