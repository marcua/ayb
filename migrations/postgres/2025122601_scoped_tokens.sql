-- Add scope fields to api_token (all nullable for backward compatibility)
ALTER TABLE api_token ADD COLUMN database_id INT REFERENCES database(id);
ALTER TABLE api_token ADD COLUMN query_permission_level SMALLINT;

-- Add OAuth/app metadata
ALTER TABLE api_token ADD COLUMN app_name VARCHAR(255);
-- Note: Using nullable timestamp, created_at will be set by application code
ALTER TABLE api_token ADD COLUMN created_at TIMESTAMP;
ALTER TABLE api_token ADD COLUMN expires_at TIMESTAMP;
ALTER TABLE api_token ADD COLUMN revoked_at TIMESTAMP;
