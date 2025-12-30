#!/bin/bash
set -e

# Setup environment
export TEST_DIR="functional_test_run"
rm -rf "$TEST_DIR"
mkdir "$TEST_DIR"
cd "$TEST_DIR"

# Ensure we have a valid environment/config
# We assume 'buckets setup' was run or logic falls back to global config which works.
# We will use 'buckets init' and rely on global config if present.

echo "=== Init ==="
buckets init test_repo
cd test_repo

echo "=== Create Bucket ==="
buckets create test_bucket
cd test_bucket

echo "=== Commit ==="
echo "v1" > file.txt
buckets commit "commit v1"

echo "=== Modify ==="
echo "v2" > file.txt
buckets commit "commit v2"

echo "=== History ==="
buckets history
# Capture commit IDs
IDS=$(buckets history --json | grep "\"id\":" | cut -d '"' -f 4)
ID_V2=$(echo "$IDS" | sed -n '1p')
ID_V1=$(echo "$IDS" | sed -n '2p')

echo "Commit V1: $ID_V1"
echo "Commit V2: $ID_V2"

if [ -z "$ID_V1" ]; then
    echo "FAIL: Could not find Commit V1"
    exit 1
fi

echo "=== Revert ==="
# Revert to V1
buckets revert --commit "$ID_V1"

CONTENT=$(cat file.txt)
echo "Content after revert: $CONTENT"

if [ "$CONTENT" == "v1" ]; then
    echo "PASS: Revert successful"
else
    echo "FAIL: Revert failed. Expected 'v1', got '$CONTENT'"
    exit 1
fi

echo "=== Commit Revert ==="
buckets commit "Reverted to V1"

echo "=== Rollback ==="
echo "v3 (uncommitted)" > file.txt
buckets rollback
CONTENT=$(cat file.txt)
if [ "$CONTENT" == "v1" ]; then
    echo "PASS: Rollback successful"
else
    echo "FAIL: Rollback failed. Expected 'v1', got '$CONTENT'"
    exit 1
fi

echo "=== Stash ==="
echo "stash me" > stash.txt
buckets stash
if [ -f "stash.txt" ]; then
    echo "FAIL: Stash did not remove file"
    exit 1
else
    echo "PASS: Stash removed file"
fi

buckets stash pop
if grep -q "stash me" stash.txt; then
    echo "PASS: Stash pop successful"
else
    echo "FAIL: Stash pop failed"
    exit 1
fi

echo "ALL FUNCTIONAL TESTS PASSED"
