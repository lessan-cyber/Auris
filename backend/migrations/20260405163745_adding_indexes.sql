-- Add migration script here
-- Indexes
CREATE INDEX idx_fingerprints_hash ON fingerprints(hash);
CREATE INDEX idx_jobs_queue ON fingerprint_jobs(status, created_at)
    WHERE status = 'queued';
