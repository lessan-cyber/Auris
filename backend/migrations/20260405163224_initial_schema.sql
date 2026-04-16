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
