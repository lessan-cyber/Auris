-- Enable extensions (Postgres 17+ has uuid_generate_v7, but let's keep ossp for v4 if needed)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create enums
CREATE TYPE track_status AS ENUM ('pending', 'fingerprinting', 'ready', 'error');
CREATE TYPE job_status AS ENUM ('queued', 'processing', 'completed', 'failed');

-- Create function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
