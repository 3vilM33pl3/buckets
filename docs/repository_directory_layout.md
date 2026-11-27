# Directory Layout
## Top level**

```bash
.\test_repo\
.\test_repo\.buckets\
.\test_repo\.buckets\buckets.db
.\test_repo\.buckets\config
.\test_repo\.buckets\db_config.toml
.\test_repo\.buckets\remote.toml
.\test_repo\.buckets\objects\
```

`.buckets\objects\` Shared content-addressable object store for every bucket. See [object storage](object_storage_and_hashing.md)

`.buckets\remote.toml` Optional path of the remote object store used by `push` and `pull`.

`.buckets\db_config.toml` Database host, port, name and user. Never contains the password.

`.buckets` Resides at the top level of the repository. Contains general information. 

`.buckets\config` Bucket repository configuration file.

`.buckets\buckets.db` Repository metadata storage. See [Database
Layout](database_layout.md)

## **Per bucket container**
Every bucket has the following layout:
```shell
.\test_repo\test_bucket\
.\test_repo\test_bucket\.b\
.\test_repo\test_bucket\.b\info
.\test_repo\test_bucket\.b\storage\
.\test_repo\test_bucket\.b\index.json
.\test_repo\test_bucket\.b\stash.json
```

`.b\index.json` Cache of size, modification time and hash per file, so status only re-hashes changed files.

`.b\stash.json` Stashes made with `buckets stash`.
`.b` At the top of the bucket, contains bucket info and storage

`.b\config` Bucket configuration file. See [Bucket Configuration](repository_configuration.md)

`.b\storage\` Legacy per-bucket object storage, read for repositories made by older versions. See [Bucket object storage](object_storage_and_hashing.md)
