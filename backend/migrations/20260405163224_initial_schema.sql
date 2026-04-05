-- Add migration script here
-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create enums
CREATE TYPE track_status AS ENUM ('pending', 'fingerprinting', 'ready', 'error');
CREATE TYPE job_status AS ENUM ('queued', 'processing', 'completed', 'failed');
