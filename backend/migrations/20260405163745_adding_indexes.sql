-- Add migration script here
-- Indexes
CREATE INDEX idx_fingerprints_hash ON fingerprints(hash);
CREATE INDEX idx_jobs_queue ON fingerprint_jobs(status, created_at)
    WHERE status = 'queued';
CREATE INDEX CONCURRENTLY idx_fingerprints_track_id ON fingerprints(track_id);
CREATE INDEX idx_fingerprints_track_id ON fingerprints(track_id);
