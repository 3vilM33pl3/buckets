# PostgreSQL Migration Notes

## Status
This branch contains a partial migration from DuckDB to PostgreSQL. Due to the extensive nature of the changes required, the migration is being done incrementally.

## Changes Made
1. **Dependencies Updated** - Removed DuckDB, added PostgreSQL and related dependencies
2. **Database Module Created** - New `postgres_db.rs` module with:
   - Support for embedded PostgreSQL
   - Support for external PostgreSQL connections
   - Connection pooling with deadpool-postgres
   - Migration system with refinery
3. **Database Configuration** - New configuration system supporting:
   - Embedded PostgreSQL (default)
   - External PostgreSQL via config file or DATABASE_URL
4. **Schema Migration** - PostgreSQL-compatible schema with UUID support

## Changes Required
The following files need to be updated to use the new PostgreSQL database:

### Core Database Operations
- [x] `src/utils/utils.rs` - Replace DuckDB connections with PostgreSQL
- [ ] `src/commands/init.rs` - Update to initialize PostgreSQL
- [ ] `src/commands/commit.rs` - Update queries to PostgreSQL syntax
- [ ] `src/data/commit.rs` - Update data operations
- [ ] `src/data/bucket.rs` - Update bucket operations

### Command Updates
- [ ] `src/commands/history.rs`
- [ ] `src/commands/revert.rs`
- [ ] `src/commands/rollback.rs`
- [ ] `src/commands/create.rs`
- [ ] `src/commands/list.rs`
- [ ] `src/commands/status.rs`

### Test Updates
All tests need to be updated to:
1. Use async/await for database operations
2. Initialize PostgreSQL instead of DuckDB
3. Handle connection pooling

## Migration Strategy

### Phase 1: Core Infrastructure (Current)
- Create PostgreSQL database module
- Setup connection management
- Create migration system

### Phase 2: Compatibility Layer
Create a compatibility layer that allows gradual migration:
```rust
// Temporary wrapper to maintain compatibility
pub fn connect_to_db() -> Result<PgConnection, BucketError> {
    // Returns a PostgreSQL connection wrapper that mimics DuckDB API
}
```

### Phase 3: Command Migration
Update commands one by one to use native PostgreSQL operations.

### Phase 4: Test Migration
Update all tests to work with PostgreSQL.

### Phase 5: Cleanup
Remove compatibility layer and optimize queries.

## Running the Migration

To complete the migration:

1. **Install PostgreSQL dependencies**:
   ```bash
   # The application will auto-install embedded PostgreSQL on first run
   ```

2. **Update runtime to async**:
   Most database operations will need to become async. Consider using tokio::runtime for synchronous contexts.

3. **Update queries**:
   - Replace `?` placeholders with `$1`, `$2`, etc.
   - Use PostgreSQL-specific features like `RETURNING`
   - Update UUID generation to use PostgreSQL's `uuid_generate_v4()`

4. **Test thoroughly**:
   ```bash
   cargo test --all-features
   ```

## Configuration Examples

### Embedded PostgreSQL (default)
No configuration needed. PostgreSQL will be downloaded and installed automatically.

### External PostgreSQL
Create `.buckets/db_config.toml`:
```toml
type = "external"
host = "localhost"
port = 5432
database = "buckets"
username = "postgres"
password = "secret"
```

Or use environment variable:
```bash
export DATABASE_URL="postgresql://postgres:secret@localhost:5432/buckets"
```

## Known Issues

1. **Edition 2024 Dependency**: The postgresql_embedded crate requires Rust edition 2024. Use nightly Rust:
   ```bash
   rustup default nightly
   ```

2. **Async Runtime**: The application needs to be converted to use async/await throughout.

3. **Query Syntax**: All queries need to be updated from DuckDB to PostgreSQL syntax.

## Next Steps

1. Create a compatibility wrapper for `connect_to_db()` that returns a PostgreSQL connection
2. Update the init command to setup PostgreSQL
3. Gradually migrate each command to use PostgreSQL
4. Update tests
5. Remove DuckDB references completely
