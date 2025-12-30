#!/bin/bash
set -e

# Setup environment
export TEST_DIR="manual_test_run"
rm -rf "$TEST_DIR"
mkdir "$TEST_DIR"
cd "$TEST_DIR"

echo "=== TC001: Setup Command Testing - Global Configuration ==="
# Mocking user input for interactive setup
echo -e "\n\n" | buckets setup
if [ -f "$HOME/.buckets_config.toml" ]; then
    echo "PASS: Config file created"
else
    echo "FAIL: Config file not created"
    exit 1
fi

echo "=== TC005: Repository Initialization - Default (Embedded PostgreSQL) ==="
buckets init test_repo_embedded
if [ -d "test_repo_embedded/.buckets/postgres_data" ]; then
    echo "PASS: Embedded DB initialized"
else
    echo "FAIL: Embedded DB directory missing"
    exit 1
fi

cd test_repo_embedded

echo "=== TC008: Bucket Creation ==="
buckets create test_bucket
if [ -d "test_bucket/.b" ]; then
    echo "PASS: Bucket created"
else
    echo "FAIL: Bucket directory missing"
    exit 1
fi

echo "=== TC009: File Commit Operations ==="
cd test_bucket
echo "This is a test file" > test_file.txt
buckets commit "Add test file"
if [ -d ".b/storage" ]; then
    echo "PASS: Storage directory created"
else
    echo "FAIL: Storage directory missing"
    exit 1
fi

# Verify commit history
if buckets history --json | grep -q "Add test file"; then
    echo "PASS: Commit recorded"
else
    echo "FAIL: Commit not recorded"
    exit 1
fi

echo "=== TC011: Status and File Tracking ==="
echo "New file" > anchor.blend
buckets commit "new anchor"
echo "Modified content" > anchor.blend
touch rudder.blend
STATUS_OUTPUT=$(buckets status)
if echo "$STATUS_OUTPUT" | grep -q "Modified: anchor.blend" && echo "$STATUS_OUTPUT" | grep -q "New: rudder.blend"; then
    echo "PASS: Status tracking correct"
else
    echo "FAIL: Status tracking incorrect: $STATUS_OUTPUT"
    exit 1
fi

echo "=== TC012: Rollback Functionality ==="
buckets rollback
if grep -q "New file" anchor.blend; then
    echo "PASS: File rolled back to committed state"
else
    echo "FAIL: File rollback failed"
    exit 1
fi

echo "=== TC013: Revert Command Testing ==="
echo "original content" > revert_test.txt
buckets commit "Add revert test file"
echo "modified content" > revert_test.txt
buckets commit "Modify revert test file"

# Get previous commit ID (skipping the current one)
# We have 4 commits now: "Add test file", "new anchor", "Add revert test file", "Modify revert test file"
# We want to revert to "Add revert test file" (2nd most recent)
TARGET_COMMIT=$(buckets history --json | grep "\"id\":" | sed -n '2p' | cut -d '"' -f 4)
echo "Reverting to commit: $TARGET_COMMIT"

buckets revert "$TARGET_COMMIT"
# Check if content matches the committed state (original content)
# Wait, revert restores ALL files to that state? Or just specific file if specified?
# The manual test says `buckets revert "$COMMIT_ID"`.
# Let's check `buckets revert --help` or docs. Docs say `buckets revert <file>` or `buckets rollback`.
# Manual TC013 says `buckets revert "$COMMIT_ID"`. This implies reverting the WHOLE bucket state?
# Let's verify behavior. If it fails, we note it.

# Actually, the command in TC013 is `buckets revert "$COMMIT_ID"`.
# If I look at the `revert` command help/code earlier (I didn't view revert code deeply), I should assume the test plan is correct or discover a bug.
# Wait, `buckets revert <file>` was in the key features list. `buckets revert <commit_id>` might be a different mode.
# Let's try it.

if buckets revert "$TARGET_COMMIT"; then
    if grep -q "original content" revert_test.txt; then
        echo "PASS: Revert successful"
    else
        echo "FAIL: Revert ran but file content incorrect"
        echo "Expected 'original content', got:"
        cat revert_test.txt
        exit 1
    fi
else
    echo "FAIL: Revert command failed"
    exit 1
fi


echo "All manual tests passed successfully!"
