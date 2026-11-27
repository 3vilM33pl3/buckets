# Stash command

Put uncommitted changes aside and bring the bucket back to its last commit.

## Usage
```bash
buckets stash save [-m <message>]   # also plain `buckets stash`
buckets stash list [--json]
buckets stash pop [--force]
buckets stash drop
```
Run inside a bucket.

## Behaviour
- `save` stores the contents of new and modified files in the shared object
  store, records the stash in `.b/stash.json`, then resets the working tree to
  the last commit: new files are removed, modified files restored and deleted
  files brought back. Removals happen before restores, so a path that changed
  between file and directory is handled.
- `pop` re-applies the newest stash and removes it from the list. It refuses
  when the bucket has uncommitted changes, unless `--force` is given.
- `drop` removes the newest stash without applying it.
- Stashes are local to the machine; they are not stored in the database.
