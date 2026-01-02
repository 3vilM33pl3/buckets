-- Expectations table
CREATE TABLE expectations (
    id UUID PRIMARY KEY,
    bucket_id UUID NOT NULL, -- The "expecting" bucket (Consumer)
    target_bucket_id UUID,   -- The "fulfilling" bucket (Producer). NULL for intrinsic.
    description TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'met')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (bucket_id) REFERENCES buckets (id),
    FOREIGN KEY (target_bucket_id) REFERENCES buckets (id)
);

-- Pebbles table
CREATE TABLE pebbles (
    id UUID PRIMARY KEY,
    description TEXT NOT NULL, -- Copy of expectation description or unique burden desc
    origin_bucket_id UUID NOT NULL, -- The original source of this burden
    current_bucket_id UUID NOT NULL, -- Where the pebble currently resides
    original_pebble_id UUID, -- If copied, reference to the pebble it was copied from (upstream)
    created_via_expectation_id UUID, -- The direct expectation that caused this pebble's creation/copy
    status TEXT NOT NULL CHECK (status IN ('active', 'resolved')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (origin_bucket_id) REFERENCES buckets (id),
    FOREIGN KEY (current_bucket_id) REFERENCES buckets (id),
    FOREIGN KEY (created_via_expectation_id) REFERENCES expectations (id),
    FOREIGN KEY (original_pebble_id) REFERENCES pebbles (id)
);
