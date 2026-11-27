# Finalize command

Resolve active pebbles in a bucket. When a pebble created by an expectation in
the same bucket is resolved, that expectation is marked `met`.

## Usage
```bash
buckets finalize [<bucket>]                          # pick from a menu
buckets finalize [<bucket>] --all                    # resolve every active pebble
buckets finalize [<bucket>] --pebble <id or description> [--pebble ...]
```
Without `<bucket>`, the bucket in the current directory is used.

## Scripts and agents
The menu needs a terminal. Without one (CI, scripts, agents), `finalize` fails
with `No terminal to prompt on. Use --all or --pebble <id or description>.`
instead of hanging. `--pebble` matches a pebble's id or its exact description,
can be repeated, and fails if nothing matches.

## JSON
With `--json`, one document is printed once all pebbles are resolved:
```json
{"resolved": [{"id": "…", "description": "…"}]}
```
`resolved` is empty when there was nothing to resolve.
