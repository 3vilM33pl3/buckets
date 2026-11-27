# Expect command

Record an expectation: something a bucket expects to receive, either from
another bucket or from itself.

## Usage
```bash
buckets expect "<description>" [--from <bucket>] [--in-bucket <bucket>] [--no-similarity]
```
- `--from <bucket>`: the bucket expected to deliver. A pebble is created there.
- `--in-bucket <bucket>`: the bucket that holds the expectation (default: the
  current bucket).
- `--no-similarity`: skip the duplicate check.

## Duplicate check
When pgvector is available, the description is embedded with
`all-MiniLM-L6-v2` (downloaded once, about 90 MB) and compared with existing
expectations. Matches above 85% similarity are listed as a warning; the
expectation is still created.

The check is skipped, with no model download, when any of these is true:
- `--no-similarity` is given,
- `BUCKETS_NO_EMBEDDINGS=1` (or `true`) is set,
- the database has no pgvector support. Expectations are then stored without
  an embedding.

## JSON
With `--json`, stdout holds one document:
```json
{"expectation": "<id>", "description": "<description>"}
```
Similarity warnings go to stderr, so stdout stays parseable.
