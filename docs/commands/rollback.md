# Rollback Command

The `rollback` command discards uncommitted changes in the current bucket and restores files to match their state in the most recent commit. This is useful when you want to undo local modifications and return to a known good state.

## Usage

Rollback all modified files in the bucket:
```shell
buckets rollback
```

Rollback changes to a specific file:
```shell
buckets rollback --path <file_path>
# or
buckets rollback -p <file_path>
```

## Arguments

- `--path` / `-p` - **Optional.** Path to a specific file to rollback. If not specified, rolls back all modified files in the bucket.

## Behavior

### Rolling Back All Files

When executed without arguments, the command:

1. **Validates Environment**:
   - Checks that you're in a buckets repository
   - Verifies you're in a valid bucket directory
   
2. **Analyzes Changes**:
   - Lists all files currently in the bucket
   - Loads the most recent commit
   - Compares current files with the committed versions
   
3. **Performs Rollback**:
   - Only processes files that have been modified (same path but different hash)
   - Restores each modified file from the `.b/storage/` directory
   - Skips the operation if no changes are detected
   
4. **Output Messages**:
   - "No files in bucket" - If the bucket is empty
   - "No changes detected. Rollback cancelled." - If files match the last commit
   - "No previous commit found." - If there's no commit history

### Rolling Back a Single File

When executed with a file path, the command:

1. **Validates File**:
   - Checks that the file exists
   - Verifies the file path is valid UTF-8
   
2. **Compares with Last Commit**:
   - Calculates the current file's hash
   - Searches for the file in the last commit
   - Verifies that both the name and hash match
   
3. **Restores File**:
   - If found with matching hash, restores from storage
   - Returns an error if the file isn't in the previous commit

## Error Conditions

The command will fail with an error if:
- Not executed within a buckets repository
- Not executed within a valid bucket directory
- No previous commit exists to rollback to
- The specified file doesn't exist (when using `--path`)
- The specified file path contains invalid UTF-8 characters
- The specified file wasn't found in the previous commit
- File restoration fails due to I/O errors

## Difference from Revert

While both commands restore files from commits, they have different use cases:

- **`rollback`**: Discards uncommitted local changes, restoring files to exactly match the last commit. Only works with the most recent commit.
- **`revert`**: Retrieves a specific file from any commit in history and overwrites the current version, regardless of local changes.

## Examples

### Rollback all changes in the current bucket
```shell
$ buckets rollback
```
This will restore all modified files to their state in the last commit.

### Rollback changes to a specific file
```shell
$ buckets rollback --path src/main.rs
```
This will restore only `src/main.rs` to its state in the last commit.

### Rollback when no changes exist
```shell
$ buckets rollback
No changes detected. Rollback cancelled.
```

### Rollback in an empty bucket
```shell
$ buckets rollback
No files in bucket
```

### Rollback with no commit history
```shell
$ buckets rollback
Error: No previous commit found.
```

## Important Notes

- **Modified Files Only**: The rollback operation only affects files that have been modified (different hash). New files that haven't been committed are not removed, and deleted files are not restored.
- **Hash-Based Comparison**: Files are compared using their content hash (BLAKE3), not timestamps or other metadata.
- **Storage Location**: Files are restored from compressed versions stored in `.b/storage/` using the file's hash as the filename.
- **Single Commit Limitation**: This command only works with the most recent commit. To restore from older commits, use the `revert` command.
- **Atomic Operation**: Each file is restored individually. If one file fails to restore, others may still be successfully restored.

## Technical Details

The rollback process uses the following logic:
1. For single file rollback: Verifies the file exists and matches a file in the last commit (by name and hash)
2. For full rollback: Uses `CommitStatus::Modified` to identify which files need restoration
3. Files are restored using the `CommittedFile::restore()` method which handles decompression and file placement
4. The command continues processing even if individual file restorations fail (errors are logged but don't stop the operation)