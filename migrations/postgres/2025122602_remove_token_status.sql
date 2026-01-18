-- Migrate existing revoked tokens to use revoked_at timestamp
-- Set revoked_at to created_at for tokens that have status=1 (revoked) but no revoked_at
UPDATE api_token SET revoked_at = COALESCE(created_at, CURRENT_TIMESTAMP) WHERE status = 1 AND revoked_at IS NULL;

-- Drop the status column
ALTER TABLE api_token DROP COLUMN status;
