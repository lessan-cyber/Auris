-- Tracks table
CREATE TABLE tracks (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    artist TEXT,
    duration_secs FLOAT NOT NULL CHECK (duration_secs >= 0),
    object_key TEXT NOT NULL,
    file_hash TEXT UNIQUE,
    status track_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Trigger for automatic updates
CREATE TRIGGER trg_set_updated_at
BEFORE UPDATE ON tracks
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();
