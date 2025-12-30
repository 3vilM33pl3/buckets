# buckets commit

## SYNOPSIS
```
buckets commit MESSAGE
```

## DESCRIPTION
Record changes to files in a bucket by creating a snapshot of the current state. The commit command captures all modified files in the bucket, creates BLAKE3 hashes for change detection, compresses and stores file contents, and records metadata in the PostgreSQL database.

Each commit represents a point-in-time snapshot that can be reverted to later. The command performs intelligent change detection by comparing current file hashes against the previous commit.

## ARGUMENTS
- `MESSAGE` - Commit message describing the changes (required)

## OPTIONS
- `-v, --verbose` - Enable verbose output showing detailed processing information
- `--json` - Output results in JSON format for programmatic use
- `-h, --help` - Show help information

## BEHAVIOR

### File Discovery
The commit operation scans the current bucket directory recursively, processing all files except:
- Files in the `.b/` directory (bucket metadata)
- Hidden files starting with `.` (except `.b/`)
- Temporary files and directories

### Change Detection
For each commit:
1. **First Commit**: All files are processed and stored
2. **Subsequent Commits**: Only files that have changed (different BLAKE3 hash) are processed
3. **No Changes**: If no files have changed since the last commit, the operation is cancelled

### File Processing
For each changed file:
1. Calculate BLAKE3 hash for content verification
2. Compress file using zstd compression
3. Store compressed file in `.b/storage/` directory
4. Record file metadata in PostgreSQL database

### Database Operations
Each commit creates records in:
- `commits` table: Commit metadata with UUID, bucket ID, message, and timestamp
- `files` table: File records linking to the commit with path and hash information

## EXAMPLES

### Basic Commit
```bash
# Commit all changes with a descriptive message
buckets commit "Add player character models and textures"
```

### Verbose Commit
```bash
# Show detailed processing information during commit
buckets commit -v "Fixed lighting issues in level 3"
```

### JSON Output
```bash
# Get commit results in JSON format
buckets commit --json "Updated animation files"
```

## FILE STRUCTURE
After a successful commit, the bucket structure includes:
```
bucket_name/
├── your_files.txt           # Your working files
├── assets/                  # Your asset directories
└── .b/                      # Bucket metadata (hidden)
    ├── info                 # Bucket configuration
    ├── storage/             # Compressed file storage
    │   └── [hash_files]     # Zstd-compressed file contents
    └── database/            # PostgreSQL database files
```

## DATABASE SCHEMA
The commit operation interacts with these PostgreSQL tables:

### commits table
```sql
CREATE TABLE commits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bucket_id UUID NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### files table
```sql
CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    commit_id UUID NOT NULL,
    file_path TEXT NOT NULL,
    hash TEXT NOT NULL,
    FOREIGN KEY (commit_id) REFERENCES commits(id)
);
```

## EXIT STATUS
- `0` - Success: Changes committed successfully
- `1` - Error: No commitable files found, database error, or I/O error
- `2` - Cancelled: No changes detected since last commit

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

#### No Bucket Found
```
Error: Not currently in a bucket directory
```
**Solution**: Run the command from within a bucket directory created with `buckets create`.

#### No Commitable Files
```
Error: No commitable files found in bucket.
```
**Solution**: Add files to the bucket directory before committing.

#### Database Connection Failed
```
Error: Failed to connect to PostgreSQL database
```
**Solution**: Ensure PostgreSQL is running or check connection settings.

#### File Access Errors
```
Error: Permission denied accessing file
```
**Solution**: Ensure proper file permissions for reading files in the bucket.

#### Storage Space Issues
```
Error: No space left on device
```
**Solution**: Free up disk space in the bucket directory or storage location.

### Troubleshooting

1. **Performance Issues with Large Files**
   - The commit operation processes files sequentially
   - Large files (>100MB) may take significant time to hash and compress
   - Consider splitting large assets into smaller chunks

2. **Memory Usage**
   - BLAKE3 hashing is performed in memory
   - Very large files may require significant RAM
   - Monitor memory usage with `top` or `htop` during commits

3. **Compression Efficiency**
   - zstd compression works best with text and uncompressed data
   - Already compressed files (images, videos) may not compress significantly
   - Binary files typically have moderate compression ratios

## PERFORMANCE CONSIDERATIONS

### Hashing Performance
- BLAKE3 is optimized for speed and uses SIMD instructions when available
- Multi-core systems will see better performance for multiple files
- Solid-state drives significantly improve I/O performance

### Compression Performance
- zstd provides good balance of compression ratio and speed
- Compression level is optimized for game asset workflows
- Network storage may impact compression performance

### Database Performance
- PostgreSQL uses write-ahead logging for commit durability
- Batch operations improve performance for commits with many files
- Regular database maintenance improves long-term performance

## SECURITY CONSIDERATIONS

### File Content Security
- File contents are stored compressed but not encrypted
- Sensitive files should use external encryption before committing
- The `.b/storage/` directory contains all committed file data

### Hash Integrity
- BLAKE3 hashes provide cryptographic verification of file integrity
- Hash collisions are computationally infeasible
- Corrupted files can be detected through hash mismatches

### Database Security

- External PostgreSQL should use proper authentication and SSL
- Connection credentials should be stored securely

## INTEGRATION WITH OTHER COMMANDS

### Workflow Integration
```bash
# Check what will be committed
buckets status

# Commit changes
buckets commit "Descriptive message"

# View commit history
buckets history

# Revert if needed
buckets revert [commit-hash]
```

### Expectation Management
```bash
# Set expectations before committing
buckets expect "Complete character animations"

# Commit progress
buckets commit "Added 50% of character animations"

# Check expectation status
buckets check
```

## MIGRATION NOTES

### From DuckDB (v0.1.x)
The commit system was migrated from DuckDB to PostgreSQL in v0.2.0:
- All commit functionality remains the same
- Database performance improved significantly
- SQL queries are more standard and maintainable
- Existing buckets need migration using the migration tools

### Async Runtime
The commit operation uses Tokio async runtime for database operations:
- Database connections are managed asynchronously
- I/O operations are non-blocking where possible
- Error handling preserves stack traces for debugging

## SEE ALSO
- `buckets-init(1)` - Initialize a new bucket repository
- `buckets-create(1)` - Create a new bucket
- `buckets-status(1)` - Show bucket status and pending changes
- `buckets-history(1)` - View commit history
- `buckets-revert(1)` - Revert to a previous commit
- `buckets-rollback(1)` - Rollback recent commits