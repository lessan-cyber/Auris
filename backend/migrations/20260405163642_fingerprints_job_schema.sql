-- Add migration script here
-- Jobs queue table
CREATE TABLE fingerprint_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    track_id UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    status job_status NOT NULL DEFAULT 'queued',
    attempts INTEGER DEFAULT 0 CHECK (attempts >= 0),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
