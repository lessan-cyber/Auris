-- Create function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to automatically update updated_at on row updates
CREATE TRIGGER trg_set_updated_at
BEFORE UPDATE ON tracks
FOR EACH ROW
EXECUTE FUNCTION set_updated_at();