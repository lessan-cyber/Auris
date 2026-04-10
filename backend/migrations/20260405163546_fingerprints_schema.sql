-- Add migration script here
-- Fingerprints table (the big one)
CREATE TABLE fingerprints (
    hash BIGINT NOT NULL,
    track_id UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    offset_ms INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (hash, track_id, offset_ms)
);
