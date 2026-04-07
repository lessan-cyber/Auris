-- Add updated_at to fingerprint_jobs
ALTER TABLE fingerprint_jobs ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Apply trigger for automatic updates
CREATE TRIGGER trg_set_jobs_updated_at
BEFORE UPDATE ON fingerprint_jobs
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();
