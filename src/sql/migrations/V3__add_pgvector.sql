-- Enable the pgvector extension to work with embeddings
CREATE EXTENSION IF NOT EXISTS vector;

-- Add embedding column to expectations table
-- Using 384 dimensions to match the all-MiniLM-L6-v2 model
ALTER TABLE expectations ADD COLUMN embedding vector(384);

-- Create an HNSW index for faster similarity search
-- m=16 and ef_construction=64 are reasonable defaults
CREATE INDEX ON expectations USING hnsw (embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64);
