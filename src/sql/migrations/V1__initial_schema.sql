-- Initial schema for buckets database

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS buckets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS commits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bucket_id UUID NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (bucket_id) REFERENCES buckets (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS files (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    commit_id UUID NOT NULL,
    file_path TEXT NOT NULL,
    hash TEXT NOT NULL,
    FOREIGN KEY (commit_id) REFERENCES commits (id) ON DELETE CASCADE,
    UNIQUE (commit_id, file_path, hash)
);

-- Create indexes for better query performance
CREATE INDEX idx_commits_bucket_id ON commits(bucket_id);
CREATE INDEX idx_commits_created_at ON commits(created_at);
CREATE INDEX idx_files_commit_id ON files(commit_id);
CREATE INDEX idx_files_hash ON files(hash);