-- Add migration script here
ALTER TABLE tracks ADD COLUMN IF NOT EXISTS file_hash TEXT;
CREATE INDEX idx_tracks_file_hash ON tracks(file_hash)
WHERE file_hash IS NOT NULL;
