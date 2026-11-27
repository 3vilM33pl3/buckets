-- Commits written from this version on record every file in the bucket
-- (a full manifest), not only the files that changed. Older commits keep
-- manifest = FALSE and are folded together when history is read.
ALTER TABLE commits ADD COLUMN IF NOT EXISTS manifest BOOLEAN NOT NULL DEFAULT FALSE;

-- Exclusive locks for files that cannot be merged (.uasset, .blend, ...).
CREATE TABLE IF NOT EXISTS locks (
    bucket_id UUID NOT NULL REFERENCES buckets (id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    owner TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bucket_id, file_path)
);
