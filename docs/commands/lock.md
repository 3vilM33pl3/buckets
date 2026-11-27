# Lock, unlock and locks commands

Binary assets such as `.uasset`, `.umap` and `.blend` cannot be merged, so two
people editing the same file loses work. Locks say who is working on a file.

## Usage
```bash
buckets lock <file>...            # take the lock
buckets unlock <file>...          # release your lock
buckets unlock --force <file>...  # release someone else's lock
buckets locks [--json]            # list locks in the current bucket
```
Run these inside a bucket. Paths may be relative to the current directory;
`.` and `..` are resolved, and paths outside the bucket are refused.

## Behaviour
- Locks are stored in PostgreSQL (`locks` table, one row per bucket and file),
  so everyone sharing the database sees them.
- The owner is `BUCKETS_USER` if set, otherwise `user@host`.
- `lock` reports `locked`, `already locked by you`, or `held by someone else`,
  and exits with an error if any file is held by someone else.
- `unlock` of a lock held by someone else fails unless `--force` is given.
- `buckets commit` refuses to commit a changed file that is locked by someone
  else. Nothing is written in that case.
- Deleting a bucket removes its locks.

## JSON
`lock`, `unlock` and `locks` accept `--json`. `locks --json` returns
`[{"bucket_id", "file_path", "owner", "created_at"}]`.
