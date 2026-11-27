-- Optional pgvector support for duplicate-expectation detection.
-- If the extension is not installed on the server, the migration still
-- succeeds and buckets runs without similarity search.
DO $$
BEGIN
    CREATE EXTENSION IF NOT EXISTS vector;
    -- 384 dimensions to match the all-MiniLM-L6-v2 model
    ALTER TABLE expectations ADD COLUMN IF NOT EXISTS embedding vector(384);
    -- HNSW index for faster similarity search (m=16, ef_construction=64)
    CREATE INDEX IF NOT EXISTS expectations_embedding_idx
        ON expectations USING hnsw (embedding vector_cosine_ops)
        WITH (m = 16, ef_construction = 64);
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'pgvector is not available (%): similarity search is disabled', SQLERRM;
END
$$;
