# Check command

Show a bucket's expectations and active pebbles, and whether every expectation
is met.

## Usage
```bash
buckets check [<bucket>] [--json]
```
Without `<bucket>`, the bucket in the current directory is used.

## JSON
```json
{
  "expectations": [
    {"id": "<id>", "description": "…", "status": "pending|met", "from": "<bucket or null>"}
  ],
  "active_pebbles": [
    {"id": "<id>", "description": "…", "origin": "<origin bucket name>"}
  ],
  "all_met": true
}
```
`all_met` is true when every expectation has status `met`.
