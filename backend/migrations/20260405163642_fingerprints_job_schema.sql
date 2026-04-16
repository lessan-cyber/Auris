-- Jobs queue table
CREATE TABLE fingerprint_jobs (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    track_id UUID NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    status job_status NOT NULL DEFAULT 'queued',
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Apply trigger for automatic updates
CREATE TRIGGER trg_set_jobs_updated_at
BEFORE UPDATE ON fingerprint_jobs
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();
