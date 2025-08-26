# Revert Command

The `revert` command restores a specific file from a previous commit to your working directory. Unlike `rollback` which discards uncommitted changes, `revert` retrieves a file from the commit history and overwrites the current version.

## Usage

Restore a file from the most recent commit:
```shell
buckets revert <file_path>
```

Restore a file from a specific commit:
```shell
buckets revert <file_path> --commit <commit_id>
# or
buckets revert <file_path> -c <commit_id>
```

## Arguments

- `<file_path>` - **Required.** Path to the file to restore. Can be relative or absolute.
- `--commit` / `-c` - **Optional.** The commit ID to restore the file from. If not specified, restores from the most recent commit.

## Behavior

1. **File Lookup**: The command searches for the specified file in the commit history
   - If a commit ID is provided, looks for the file in that specific commit
   - Otherwise, searches for the file in the most recent commit

2. **Restoration Process**:
   - Retrieves the compressed file from the `.b/storage/` directory using the file's hash
   - Decompresses the file using zstd decompression
   - Creates parent directories if they don't exist
   - Overwrites the existing file (if present) with the restored version

3. **Error Conditions**:
   - Returns an error if not in a buckets repository
   - Returns an error if not in a valid bucket directory
   - Returns an error if the file doesn't exist in the specified commit
   - Returns an error if the file has never been committed

## Difference from Rollback

While both commands restore files from commits, they serve different purposes:

- **`revert`**: Retrieves a specific file from commit history (any commit) and replaces the current version
- **`rollback`**: Discards uncommitted changes and restores files to match the last commit exactly

## Examples

### Restore a single file from the last commit
```shell
buckets revert src/main.rs
```
Output: `Restored src/main.rs from latest commit`

### Restore a file from a specific commit
```shell
buckets revert config.toml --commit abc123def
```
Output: `Restored config.toml from commit abc123def`

### Restore a file in a subdirectory
```shell
buckets revert docs/readme.md
```
Output: `Restored docs/readme.md from latest commit`

## Notes

- The command preserves the file structure and creates necessary parent directories
- Files are stored in compressed format (zstd) in the `.b/storage/` directory
- The original file (if it exists) is completely replaced by the restored version
- This command only affects the specified file, leaving other files unchanged