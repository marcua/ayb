-- Add scope fields to api_token (all nullable for backward compatibility)

-- Database scope: NULL means token works for all databases the user owns/has access to
ALTER TABLE api_token ADD COLUMN database_id INT REFERENCES database(id);

-- Permission level cap: NULL means no cap (use user's permission level)
-- Uses QueryMode values: 0 = ReadOnly, 1 = ReadWrite
ALTER TABLE api_token ADD COLUMN query_permission_level SMALLINT;

-- Track which entity authorized this token (for shared databases, may differ from entity_id)
ALTER TABLE api_token ADD COLUMN granted_by INT REFERENCES entity(id);

-- OAuth/app metadata
ALTER TABLE api_token ADD COLUMN app_name VARCHAR(255);
ALTER TABLE api_token ADD COLUMN app_origin_url VARCHAR(255);

-- Timestamps
-- Note: created_at defaults to now for new tokens, but existing tokens will have NULL
ALTER TABLE api_token ADD COLUMN created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;
ALTER TABLE api_token ADD COLUMN expires_at TIMESTAMP;
ALTER TABLE api_token ADD COLUMN revoked_at TIMESTAMP;
