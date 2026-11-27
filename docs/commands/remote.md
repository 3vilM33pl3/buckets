# Remote, push and pull commands

Commit history lives in PostgreSQL; file contents live in the repository's
object store `.buckets/objects/`. A *remote* is a second object store in a
directory, typically on a NAS, that every machine can reach. `push` and `pull`
copy objects between the two, so machines sharing the database can share files.

## Usage
```bash
buckets remote set <path>   # create the directory and store it in .buckets/remote.toml
buckets remote show
buckets remote unset
buckets push [--json]       # upload objects referenced by any commit
buckets pull [--force] [--json]
```
`BUCKETS_REMOTE=<path>` overrides the configured remote.

## push
Copies every object referenced by a commit of a bucket in this repository
(buckets are found at any depth) from the local store, or the legacy
`.b/storage/`, to the remote. Objects already on the remote are skipped.
`--json` returns `{"remote", "objects_copied", "objects_already_there", "missing_locally"}`.

## pull
Run inside a bucket to update that bucket, or at the repository root to update
every bucket. For each bucket it brings the working files to the latest commit:
- Missing objects are fetched from the remote.
- Files that are not in the latest commit, but were committed before, are
  removed (empty directories are pruned) before new files are written, so a
  path that changed between file and directory updates cleanly.
- Files the bucket never committed are untracked and always kept.
- A file with local edits is a **conflict**: it is left untouched and the
  command fails, unless `--force` is given.

`--json` returns one entry per bucket:
`{"bucket", "commit_id", "updated", "removed", "conflicts", "objects_fetched"}`.

The remote must be reachable; if the path does not exist (for example the NAS
is not mounted), the command fails instead of creating an empty store.
