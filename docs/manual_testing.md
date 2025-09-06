# Manual Test Plan for Buckets CLI

## Table of Contents

1. [Test Environment Setup](#test-environment-setup)
2. [Setup Command Tests](#setup-command-tests)
3. [Core Functionality Tests](#core-functionality-tests)
4. [Information Command Tests](#information-command-tests)
5. [Expectation Management Tests](#expectation-management-tests)
6. [Database Management Tests](#database-management-tests)
7. [Integration and Performance Tests](#integration-and-performance-tests)
8. [Error Handling Tests](#error-handling-tests)
9. [Test Automation Scripts](#test-automation-scripts)
10. [Test Execution Guidelines](#test-execution-guidelines)

## Test Environment Setup

### Prerequisites
- Rust toolchain installed (1.70+)
- PostgreSQL client tools (psql, pg_dump)
- Git (for repository management)
- Standard Unix tools (curl, tar, gzip)
- jq (for JSON validation)
- Minimum 2GB free disk space for testing

### Installation Methods

#### Option 1: Direct Installation
```bash
cd buckets
cargo install --path .
buckets --version
```

#### Option 2: Development Build
```bash
cd buckets
cargo build --release
./target/release/buckets --version
```

#### Option 3: Windows-specific Setup
```powershell
cd buckets
cargo install --path .
Get-Command buckets.exe
Set-Alias buckets "C:\Users\WindowsUser\.cargo\bin\buckets.exe"
# PostgreSQL is automatically managed by buckets
buckets --version
```

---

## Setup Command Tests

### TC001: Setup Command Testing - Global Configuration

**Priority:** Critical
**Category:** Configuration Management

**Objective:** Verify global configuration setup functionality

**Preconditions:** Clean environment, no existing global configuration

**Test Steps:**
```bash
# Test setup command help
buckets setup --help

# Test interactive setup with default values
echo -e "\n\n" | buckets setup

# Verify configuration file creation
ls -la ~/.buckets_config.toml
cat ~/.buckets_config.toml
```

**Expected Results:**
- Exit code: 0
- Configuration file created at `~/.buckets_config.toml`
- Default NTP server: "pool.ntp.org"
- No PostgreSQL connection by default
- Interactive prompts display current values

**Verification Commands:**
```bash
# Check configuration file format
buckets setup --help | grep -E "(PostgreSQL|NTP)"
cat ~/.buckets_config.toml | grep ntp_server
```

**Post-conditions:** Global configuration file exists and is valid

**Notes:** Setup command should work outside any buckets repository

---

### TC002: Setup Command Testing - PostgreSQL Configuration

**Priority:** High
**Category:** Configuration Management

**Objective:** Verify PostgreSQL connection string configuration

**Preconditions:** Setup command available

**Test Steps:**
```bash
# Configure PostgreSQL connection
echo -e "postgresql://user:pass@localhost:5432/buckets\ntime.ntp.org\n" | buckets setup

# Verify updated configuration
cat ~/.buckets_config.toml

# Test configuration persistence
echo -e "\n\n" | buckets setup
```

**Expected Results:**
- PostgreSQL connection string saved correctly
- Custom NTP server saved
- Subsequent runs show existing values
- Configuration persists between invocations

**Verification Commands:**
```bash
grep "postgresql_connection" ~/.buckets_config.toml
grep "ntp_server" ~/.buckets_config.toml
```

**Post-conditions:** Global configuration contains PostgreSQL connection string

---

### TC003: Setup Command Testing - Configuration Integration

**Priority:** Critical  
**Category:** Integration

**Objective:** Verify repository configs inherit global settings

**Preconditions:** Global configuration set (from TC002)

**Test Steps:**
```bash
# Clean up any existing test repos
rm -rf setup_test_repo

# Initialize repository and check inherited config
buckets init setup_test_repo
cd setup_test_repo
cat .buckets/config
```

**Expected Results:**
- Repository configuration inherits NTP server from global config
- Repository configuration inherits PostgreSQL connection from global config
- Local repository settings use global defaults where applicable
- No errors during initialization with global config present

**Verification Commands:**
```bash
# Verify NTP server inheritance
grep "time.ntp.org" .buckets/config || grep "pool.ntp.org" .buckets/config

# Verify PostgreSQL connection inheritance
grep "postgresql_connection" .buckets/config || echo "No PostgreSQL connection inherited"
```

**Post-conditions:** Repository uses global configuration values

**Notes:** This tests the integration between setup command and repository initialization

---

### TC004: Setup Command Testing - Error Handling

**Priority:** Medium
**Category:** Error Handling

**Objective:** Verify setup command handles errors gracefully

**Test Steps:**
```bash
# Test with invalid input/interruption
echo -e "\x03" | buckets setup || true

# Test with corrupted config file
echo "invalid toml content {[" > ~/.buckets_config.toml
buckets setup || true
```

**Expected Results:**
- Graceful handling of interrupted input
- Clear error messages for permission issues
- Recovery from corrupted configuration files
- Non-zero exit codes for error conditions

**Verification Commands:**
```bash
# Verify setup can recover from errors
rm -f ~/.buckets_config.toml
echo -e "\n\n" | buckets setup
echo $?
```

**Post-conditions:** Setup command recovers from error conditions

---

## Core Functionality Tests

### TC005: Repository Initialization - Default (Embedded PostgreSQL)

**Priority:** Critical
**Category:** Core Functionality

**Objective:** Verify repository initialization with default embedded PostgreSQL backend

**Preconditions:** Clean test environment

**Test Steps:**
```bash
buckets init test_repo_embedded
```

**Expected Results:**
- Exit code: 0
- Console output: "Bucket repository initialized successfully."
- Directory structure:
  ```
  ./test_repo_embedded/
  ./test_repo_embedded/.buckets/
  ./test_repo_embedded/.buckets/config
  ./test_repo_embedded/.buckets/database_type
  ./test_repo_embedded/.buckets/postgres_data/
  ./test_repo_embedded/.buckets/postgres_data/postgresql.conf
  ./test_repo_embedded/.buckets/postgres_data/pg_hba.conf
  ```
- Database type file contains: `embedded`
- PostgreSQL data directory initialized with proper permissions

**Database Schema Verification:**
```bash
# Connect to embedded PostgreSQL (port will be dynamically assigned)
buckets list --json  # This will verify database connectivity
```

**Post-conditions:** Repository ready for bucket creation

---

### TC006: Repository Initialization - External PostgreSQL Backend

**Priority:** High
**Category:** Core Functionality

**Objective:** Verify repository initialization with external PostgreSQL backend

**Preconditions:** 
- Clean test environment
- External PostgreSQL server running and accessible
- Environment variables set:

```bash
export DATABASE_URL="postgresql://username:password@host:port/database"
```

**Test Steps:**
```bash
buckets init test_repo_external --database external
```

**Expected Results:**
- Exit code: 0
- Console output: "Bucket repository initialized successfully."
- Directory structure:
  ```
  ./test_repo_external/
  ./test_repo_external/.buckets/
  ./test_repo_external/.buckets/config
  ./test_repo_external/.buckets/database_type
  ```
- Database type file contains: `external`
- No local PostgreSQL data directory created
- External database contains proper schema

**Database Schema Verification:**
```bash
psql -h localhost -U buckets_test -d buckets_test -c "\dt"
```
Expected tables: `buckets`, `commits`, `files`

**Post-conditions:** Repository ready for bucket creation

---

### TC007: Database Option Validation

**Priority:** High
**Category:** Input Validation

**Objective:** Verify proper validation of database type parameter

**Test Cases:**

#### TC007a: Valid Database Types
```bash
buckets init test_valid_embedded --database embedded    # Should succeed
buckets init test_valid_external --database external    # Should succeed  
buckets init test_valid_postgres --database postgres    # Should succeed
buckets init test_valid_postgresql --database postgresql # Should succeed
```

#### TC007b: Invalid Database Type
```bash
buckets init test_invalid --database mysql
```
**Expected Results:**
- Exit code: non-zero
- Error message: "Invalid database type 'mysql'. Valid options are: embedded, external, postgresql"

#### TC007c: External PostgreSQL Without Connection
```bash
# Unset environment variables
unset POSTGRES_HOST POSTGRES_PORT POSTGRES_USER POSTGRES_PASSWORD POSTGRES_DB
buckets init test_no_connection --database external
```
**Expected Results:**
- Exit code: non-zero  
- Error message indicating connection failure or missing environment variables

---

### TC008: Bucket Creation

**Priority:** Critical
**Category:** Core Functionality

**Objective:** Verify bucket creation functionality

**Preconditions:** Valid repository initialized (from TC005 or TC006)

**Test Steps:**
```bash
cd test_repo_embedded  # or test_repo_external
buckets create test_bucket
```

**Expected Results:**
- Exit code: 0
- Console output indicating successful bucket creation
- Directory structure:
  ```
  ./test_bucket/
  ./test_bucket/.b/
  ./test_bucket/.b/info
  ./test_bucket/.b/storage/
  ```
- Database verification:
  ```bash
  buckets list --json
  ```
  Expected: JSON output showing bucket with UUID, name="test_bucket", path="test_bucket"

**Post-conditions:** Bucket ready for file operations

---

### TC009: File Commit Operations

**Priority:** Critical
**Category:** Core Functionality

**Objective:** Verify file commit functionality across database backends

**Preconditions:** Bucket created (from TC008)

**Test Steps:**
```bash
cd test_bucket
echo "This is a test file" > test_file.txt
buckets commit "Add test file"
```

**Expected Results:**
- Exit code: 0
- File storage directory created: `./.b/storage/`
- Database records updated in commits and files tables
- File hash correctly stored

**Database Verification:**
```bash
# Verify commit history
buckets history --json

# Expected JSON output with commit record
# {
#   "commits": [
#     {
#       "id": "uuid-here",
#       "message": "Add test file",
#       "created_at": "2024-xx-xx",
#       "bucket_name": "test_bucket"
#     }
#   ]
# }
```

---

### TC010: Cross-Platform Compatibility

**Priority:** Medium
**Category:** Core Functionality

**Objective:** Verify functionality across different operating systems

#### TC010a: Unix/Linux Commands
```bash
cd test_bucket
touch boat.blend
echo "Blender file content" > boat.blend
buckets commit "new boat"
```

#### TC010b: Windows Commands  
```powershell
cd test_bucket
New-Item boat.blend -ItemType File
"Blender file content" | Out-File -FilePath .\boat.blend
buckets commit "new boat"
```

**Expected Results:** Consistent behavior across platforms

---

### TC011: Status and File Tracking

**Priority:** High
**Category:** Core Functionality

**Objective:** Verify status reporting functionality

**Test Steps:**
```bash
cd test_bucket
echo "New file" > anchor.blend
buckets commit "new anchor"
echo "Modified content" > anchor.blend
touch rudder.blend
buckets status
```

**Expected Results:**
```
Repository: [repository_name]
Bucket: [bucket_name]

Committed files: [count]
Modified files: [count]  
New files: [count]

Status details:
- Modified: anchor.blend  
- New: rudder.blend
```

---

### TC012: Rollback Functionality

**Priority:** High
**Category:** Core Functionality

**Objective:** Verify rollback operations

**Test Steps:**
```bash
buckets rollback
buckets status
```

**Expected Results:**
- Modified files restored to committed state
- Status shows clean working directory for committed files
- New files remain untracked

---

### TC013: Revert Command Testing

**Priority:** High
**Category:** Core Functionality

**Objective:** Verify file reversion to previous commits

**Preconditions:** Bucket with committed files and subsequent changes

**Test Steps:**
```bash
cd test_bucket
echo "original content" > revert_test.txt
buckets commit "Add revert test file"
echo "modified content" > revert_test.txt
buckets commit "Modify revert test file"
# Get commit ID from history
COMMIT_ID=$(buckets history --json | jq -r '.commits[1].id')
buckets revert $COMMIT_ID
```

**Expected Results:**
- Files restored to state at specified commit
- Working directory shows reverted file content
- Database maintains commit history integrity

---

### TC014: Stash Command Testing

**Priority:** Medium
**Category:** Core Functionality

**Objective:** Verify temporary change storage and retrieval

**Preconditions:** Bucket with uncommitted changes

**Test Steps:**
```bash
cd test_bucket
echo "stash test content" > stash_test.txt
buckets stash
# Verify file is removed from working directory
ls stash_test.txt  # Should not exist
buckets stash pop
# Verify file is restored
cat stash_test.txt  # Should show "stash test content"
```

**Expected Results:**
- Stash stores uncommitted changes
- Working directory cleaned after stash
- Pop restores changes to working directory

---

## Information Command Tests

### TC015: History Command Testing

**Priority:** Medium  
**Category:** Information Commands

**Objective:** Verify commit history retrieval functionality

**Preconditions:** Repository with multiple commits (from previous tests)

**Test Steps:**
```bash
cd test_bucket
buckets history
buckets history --json
buckets history -v
```

**Expected Results:**
- Default format shows commit history with IDs, messages, timestamps, bucket names
- JSON format provides structured data for programmatic use
- Verbose mode shows additional debug information
- Commits ordered by creation timestamp (newest first)

---

### TC016: List Command Testing

**Priority:** Medium
**Category:** Information Commands  

**Objective:** Verify bucket listing functionality

**Preconditions:** Repository with multiple buckets created

**Test Steps:**
```bash
cd test_repo_embedded
buckets create bucket_one
buckets create bucket_two
buckets list
buckets list --json
```

**Expected Results:**
- Shows all buckets with names, IDs, and relative paths
- JSON output provides structured bucket information
- Empty repositories show "No buckets found"

---

### TC017: Stats Command Testing

**Priority:** Low
**Category:** Information Commands

**Objective:** Verify repository statistics reporting

**Preconditions:** Repository with multiple buckets and commits

**Test Steps:**
```bash
buckets stats
buckets stats --json
```

**Expected Results:**
- Shows repository statistics (bucket count, commit count, file count, storage size)
- JSON format provides structured statistics data
- Accurate counts matching actual repository state

---

### TC018: JSON Output Validation

**Priority:** Medium
**Category:** Information Commands

**Objective:** Verify JSON output format consistency

**Test Steps:**
```bash
# Test all commands that support JSON output
buckets list --json | jq '.'
buckets history --json | jq '.'
buckets stats --json | jq '.'
buckets status --json | jq '.'

# Validate specific JSON schema
buckets list --json | jq -e '.buckets | type == "array"'
buckets history --json | jq -e '.commits | type == "array"'

# Test JSON with complex data
echo '{"test": "data"}' > complex.json
buckets commit "Add JSON file"
buckets history --json | jq -e '.commits[0].message == "Add JSON file"'
```

**Expected Results:**
- Valid JSON format for all commands
- Consistent field naming and structure
- Proper data types (strings, numbers, arrays, objects)
- No syntax errors when parsed
- Schema validation passes for known fields

---

## Expectation Management Tests

### TC019: Expect Command Testing

**Priority:** Medium
**Category:** Expectation Management

**Objective:** Verify expectation setting functionality

**Preconditions:** Bucket created and ready for expectations

**Test Steps:**
```bash
cd test_bucket
buckets expect "Complete character models by Friday"
buckets expect "Add 10 texture files" --priority high
```

**Expected Results:**
- Expectations stored in database
- Priority levels handled correctly
- Expectations visible in status or dedicated commands

---

### TC020: Check Command Testing

**Priority:** Medium
**Category:** Expectation Management

**Objective:** Verify expectation checking and status reporting

**Preconditions:** Expectations set (from TC019)

**Test Steps:**
```bash
buckets check
buckets check --detailed
```

**Expected Results:**
- Shows current expectation status
- Detailed mode provides additional context
- Clear indication of met/unmet expectations

---

### TC021: Link Command Testing

**Priority:** Low
**Category:** Expectation Management

**Objective:** Verify linking between expectations and commits/files

**Preconditions:** Expectations and commits exist

**Test Steps:**
```bash
buckets link expectation commit_id
buckets link expectation file_path
```

**Expected Results:**
- Creates associations between expectations and artifacts
- Links visible in status or check commands
- Proper validation of target existence

---

### TC022: Finalize Command Testing

**Priority:** Medium
**Category:** Expectation Management

**Objective:** Verify expectation finalization workflow

**Preconditions:** Expectations with completed work

**Test Steps:**
```bash
buckets finalize expectation_id
buckets finalize --all
```

**Expected Results:**
- Marks expectations as finalized/completed
- Prevents further modifications to finalized expectations
- Provides confirmation of finalization

---

## Database Management Tests

### TC023: Schema Command Testing

**Priority:** Low
**Category:** Database Management

**Objective:** Verify database schema operations

**Test Steps:**
```bash
buckets schema show
buckets schema validate
buckets schema migrate  # If applicable
```

**Expected Results:**
- Shows current database schema version
- Validates schema integrity
- Handles schema migrations if needed

---

## Integration and Performance Tests

### TC024: Performance and Load Testing

**Priority:** Medium
**Category:** Performance

**Objective:** Verify system performance under load

**Test Scenarios:**

#### TC024a: Large File Handling
```bash
# Create large test file (100MB)
dd if=/dev/zero of=large_file.bin bs=1M count=100
buckets commit "Add large file"
```

#### TC024b: Many Small Files
```bash
# Create many small files
for i in {1..1000}; do
  echo "File $i content" > "small_file_$i.txt"
done
buckets commit "Add 1000 small files"
```

#### TC024c: Deep Directory Structure
```bash
# Create deep nested directories
mkdir -p deep/nested/directory/structure/with/many/levels
echo "deep file" > deep/nested/directory/structure/with/many/levels/file.txt
buckets commit "Add deeply nested file"
```

**Expected Results:**
- Commands complete within reasonable time limits
- Memory usage remains acceptable
- No crashes or corruption with large datasets

---

### TC025: Multi-Bucket Workflow Testing

**Priority:** High
**Category:** Integration

**Objective:** Verify complex multi-bucket scenarios

**Test Steps:**
```bash
# Create multiple related buckets
buckets create assets
buckets create textures  
buckets create models
buckets create animations

# Add files to each bucket
cd assets
echo "Asset index" > index.txt
buckets commit "Add asset index"

cd ../textures
echo "Texture data" > texture.png
buckets commit "Add texture"

cd ../models
echo "Model data" > character.blend
buckets commit "Add character model"

cd ../animations
echo "Animation data" > walk.fbx
buckets commit "Add walk animation"

# Test cross-bucket operations
cd ..
buckets list
buckets history
buckets stats
```

**Expected Results:**
- All buckets function independently
- Repository-wide commands show data from all buckets
- No interference between bucket operations

---

## Error Handling Tests

### TC026: Error Handling and Edge Cases

**Priority:** High
**Category:** Error Handling

**Objective:** Verify robust error handling

#### TC026a: Duplicate Repository
```bash
buckets init existing_repo
buckets init existing_repo  # Should fail
```

#### TC026b: Operations Outside Repository
```bash
mkdir /tmp/not_a_repo
cd /tmp/not_a_repo  
buckets create test_bucket  # Should fail
```

#### TC026c: Invalid Bucket Names
```bash
buckets create ""           # Empty name
buckets create "invalid/name"  # Invalid characters
buckets create "bucket with spaces"  # Spaces in name
buckets create "very-long-bucket-name-that-exceeds-reasonable-limits"  # Too long
```

#### TC026d: Network Failure Handling (External PostgreSQL)
```bash
# With external PostgreSQL configured but server down
export POSTGRES_HOST=unreachable-server.com
buckets init test_network_failure --database external
```

#### TC026e: Disk Space Exhaustion
```bash
# Simulate low disk space conditions
# Create large file that fills available space
buckets commit "Test disk space handling"
```

#### TC026f: Permission Errors
```bash
# Test with read-only directories
chmod 444 test_readonly_bucket
buckets commit "Test permission error"
```

**Expected Results:** Appropriate error messages and non-zero exit codes for all failure scenarios

---

### TC027: Help and Documentation

**Priority:** Medium
**Category:** Error Handling

**Objective:** Verify help system functionality

**Test Cases:**
```bash
buckets --help                           # General help
buckets init --help                      # Init command help  
buckets create --help                    # Create command help
buckets commit --help                    # Commit command help
buckets setup --help                     # Setup command help
```

**Expected Results:** 
- Comprehensive help text displayed
- Database option documented for init command
- Setup command options clearly explained
- All required parameters clearly indicated

---

## Test Automation Scripts

### Quick Test Script
```bash
#!/bin/bash
# quick_test.sh - Basic functionality verification

set -e  # Exit on any error

echo "=== Buckets CLI Quick Test ==="

# Cleanup any existing test data
rm -rf test_quick_* ~/.buckets_config.toml 2>/dev/null || true

# Test 0: Global configuration setup
echo "Testing global configuration setup..."
echo -e "\ntime.example.com\n" | buckets setup
if [ -f ~/.buckets_config.toml ]; then
    echo "✅ Global configuration created"
else
    echo "❌ Global configuration failed"
    exit 1
fi

# Test 1: Repository initialization
echo "Testing repository initialization..."
buckets init test_quick_repo
cd test_quick_repo

# Test 2: Bucket creation
echo "Testing bucket creation..."
buckets create quick_bucket
cd quick_bucket

# Test 3: File operations
echo "Testing file operations..."
echo "Test content $(date)" > test_file.txt
buckets commit "Add test file"

# Test 4: Information commands
echo "Testing information commands..."
buckets status
buckets history
cd ..
buckets list

# Cleanup
echo "Cleaning up test data..."
cd ..
rm -rf test_quick_* ~/.buckets_config.toml 2>/dev/null || true

echo "✅ Quick test completed successfully!"
```

### Performance Benchmark Script
```bash
#!/bin/bash
# benchmark.sh - Performance testing

set -e

echo "=== Buckets Performance Benchmark ==="

# Cleanup
rm -rf perf_test_* 2>/dev/null || true

# Initialize repository
echo "Initializing performance test repository..."
time buckets init perf_test_repo
cd perf_test_repo
buckets create perf_bucket
cd perf_bucket

# Benchmark 1: Many small files
echo "Benchmark 1: Committing 100 small files..."
start_time=$(date +%s)
for i in {1..100}; do
  echo "Content for file $i" > "small_file_$i.txt"
done
time buckets commit "Add 100 small files"
end_time=$(date +%s)
echo "Small files commit took: $((end_time - start_time)) seconds"

# Benchmark 2: Large file
echo "Benchmark 2: Committing 10MB file..."
dd if=/dev/zero of=large_file.bin bs=1M count=10 2>/dev/null
time buckets commit "Add large file"

# Benchmark 3: Deep directory structure
echo "Benchmark 3: Deep directory structure..."
mkdir -p a/b/c/d/e/f/g/h/i/j
echo "Deep file content" > a/b/c/d/e/f/g/h/i/j/deep_file.txt
time buckets commit "Add deep file"

echo "✅ Performance benchmark completed!"
```

### Integration Test Script
```bash
#!/bin/bash
# integration_test.sh - Full workflow testing

set -e

echo "=== Buckets Integration Test ==="

# Test setup command integration
echo "Setting up global configuration..."
rm -f ~/.buckets_config.toml
echo -e "postgresql://test:test@localhost:5432/buckets_test\ntime.pool.ntp.org\n" | buckets setup
echo "✅ Global configuration set"

# Test multi-bucket workflow
rm -rf integration_test_* 2>/dev/null || true
buckets init integration_test_repo
cd integration_test_repo

# Verify configuration inheritance
if grep -q "time.pool.ntp.org" .buckets/config; then
    echo "✅ Configuration inherited from global setup"
else
    echo "⚠️ Configuration inheritance not verified"
fi

# Create project structure
buckets create assets
buckets create textures
buckets create models
buckets create animations

# Populate each bucket
echo "Populating assets bucket..."
cd assets
echo "Asset manifest v1.0" > manifest.json
echo "Project README" > README.md
buckets commit "Initial asset setup"

echo "Populating textures bucket..."
cd ../textures
echo "Texture data" > wood_texture.png
echo "Metal texture data" > metal_texture.png
buckets commit "Add initial textures"

echo "Populating models bucket..."
cd ../models
echo "Character model data" > character.blend
buckets commit "Add character model"

echo "Populating animations bucket..."
cd ../animations
echo "Walk cycle data" > walk.fbx
echo "Run cycle data" > run.fbx
buckets commit "Add basic animations"

# Test repository-wide operations
echo "Testing repository-wide operations..."
cd ..
buckets list --json > bucket_list.json
buckets history --json > commit_history.json
buckets stats --json > repo_stats.json

# Validate JSON output
if command -v jq >/dev/null; then
  echo "Validating JSON output..."
  jq '.' bucket_list.json >/dev/null
  jq '.' commit_history.json >/dev/null
  jq '.' repo_stats.json >/dev/null
  echo "✅ JSON validation passed"
fi

echo "✅ Integration test completed successfully!"
```

---

## Test Execution Guidelines

### Test Environment Setup

#### Minimum System Requirements
- **OS:** Linux, macOS, or Windows 10+
- **RAM:** 4GB minimum, 8GB recommended
- **Disk:** 2GB free space for testing
- **Network:** Required for external PostgreSQL tests

#### Required Dependencies
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install PostgreSQL client tools
# Ubuntu/Debian:
sudo apt-get install postgresql-client-common postgresql-client

# macOS:
brew install postgresql

# Install jq for JSON validation
# Ubuntu/Debian:
sudo apt-get install jq
# macOS:
brew install jq
```

#### Build Instructions
```bash
# Clone repository
git clone https://github.com/3vilM33pl3/buckets.git
cd buckets

# Build with all features
cargo build --release --all-features

# Run unit tests
cargo test

# Install for system-wide testing
cargo install --path .
```

### Test Execution Strategy

#### Phase 1: Critical Path Testing
1. **TC001** - Setup command (global configuration)
2. **TC002** - Setup PostgreSQL configuration
3. **TC003** - Setup integration with repository config
4. **TC005** - Repository initialization (embedded)
5. **TC008** - Bucket creation
6. **TC009** - File commit operations
7. **TC011** - Status and file tracking
8. **TC012** - Rollback functionality

#### Phase 2: Extended Functionality
1. **TC006** - External PostgreSQL
2. **TC015-TC018** - Information commands
3. **TC019-TC023** - Expectation and database management
4. **TC025** - Multi-bucket workflows

#### Phase 3: Performance and Edge Cases
1. **TC024** - Performance testing
2. **TC018** - JSON validation
3. **TC026-TC027** - Error handling

### Test Data Management

#### Standard Test Files
```bash
# Create standard test data directory
mkdir -p test_data
cd test_data

# Small text file
echo "Small test file content" > small.txt

# Medium binary file
dd if=/dev/urandom of=medium.bin bs=1K count=100

# Large file for performance testing
dd if=/dev/zero of=large.bin bs=1M count=50

# Nested directory structure
mkdir -p nested/deep/structure
echo "Nested file" > nested/deep/structure/file.txt

# Special characters in filename
echo "Special chars" > "file with spaces & symbols!.txt"
```

#### Test Data Cleanup
```bash
#!/bin/bash
# cleanup_test_data.sh
echo "Cleaning up test data..."
rm -rf test_* perf_test_* integration_test_* setup_test_* 2>/dev/null || true
rm -f ~/.buckets_config.toml 2>/dev/null || true
rm -f *.json *.log 2>/dev/null || true
echo "✅ Cleanup completed (including global configuration)"
```

### Pass/Fail Criteria

#### Test Result Categories
- **✅ PASS:** All expected results achieved, correct exit codes
- **❌ FAIL:** Expected results not achieved, incorrect behavior
- **⚠️ PARTIAL:** Some functionality works, minor issues present
- **🚫 BLOCKED:** Cannot execute due to environment issues
- **⏭️ SKIPPED:** Test not applicable to current configuration

#### Critical Failure Criteria
- Database corruption or data loss
- Memory leaks or crashes
- Security vulnerabilities
- Performance degradation >50% from baseline

### Test Reporting

#### Test Report Template
```markdown
# Buckets CLI Test Report

**Date:** YYYY-MM-DD
**Tester:** Name
**Version:** vX.Y.Z
**Environment:** OS, Rust version

## Summary
- Total Tests: X
- Passed: X
- Failed: X
- Blocked: X

## Critical Issues
[List any critical failures]

## Test Results
| Test Case | Status | Notes |
|-----------|--------|-------|
| TC001     | ✅     | Setup command working |
| TC002     | ✅     | PostgreSQL config OK |
| TC003     | ✅     | Config inheritance working |
| TC005     | ✅     | -     |
| TC006     | ❌     | Connection timeout |

## Performance Metrics
- Repository init: X.X seconds
- File commit (100 files): X.X seconds
- Large file (100MB): X.X seconds

## Recommendations
[Suggestions for improvements]
```

#### Automated Reporting
```bash
#!/bin/bash
# generate_report.sh
echo "Generating test report..."
echo "# Buckets CLI Test Report" > test_report.md
echo "**Date:** $(date)" >> test_report.md
echo "**Environment:** $(uname -a)" >> test_report.md
echo "**Rust Version:** $(rustc --version)" >> test_report.md
echo "**Global Config:** $(test -f ~/.buckets_config.toml && echo "Present" || echo "Not found")" >> test_report.md
# Add test results...
```

---

## Documentation Update Summary

### Recent Changes Made:

#### ✅ **Reorganized Document Structure**
- **Setup Command Tests** moved to top priority position (after environment setup)
- **Table of Contents** added with clear navigation
- **Tests categorized by command type** for better organization
- **Test cases renumbered** for logical flow

#### ✅ **Setup Command Priority**
- **TC001-TC004**: Setup command tests now appear first in testing sequence
- **Critical path testing** updated to include setup command as first step
- **Integration testing** emphasizes setup command inheritance verification

#### ✅ **Command-Based Categorization**
- **Setup Command Tests** (TC001-TC004)
- **Core Functionality Tests** (TC005-TC014)
- **Information Command Tests** (TC015-TC018)
- **Expectation Management Tests** (TC019-TC022)
- **Database Management Tests** (TC023)
- **Integration and Performance Tests** (TC024-TC025)
- **Error Handling Tests** (TC026-TC027)

#### ✅ **Enhanced Test Coverage**
- **Total Commands**: 16 (including setup)
- **Setup Command**: 4 dedicated test cases covering all aspects
- **Configuration inheritance**: Explicitly tested in TC003
- **Error handling**: Comprehensive coverage including setup command

#### ✅ **Improved Navigation and Usability**
- **Table of Contents**: Links to all major sections
- **Logical test ordering**: Setup → Core → Information → Advanced
- **Clear test priorities**: Critical setup tests first
- **Consistent formatting**: All test cases follow standard structure

### Test Coverage Status:
- **Total Commands**: 16 (100% coverage)
- **Setup Command Tests**: 4 comprehensive test cases
- **Critical Path Tests**: 8 test cases (including setup)
- **Total Test Cases**: 27 (TC001-TC027)

### Test Execution Priority:
1. **Setup Command Tests** (TC001-TC004) - **Must run first**
2. **Core Functionality** (TC005-TC014)
3. **Information Commands** (TC015-TC018)
4. **Advanced Features** (TC019-TC027)