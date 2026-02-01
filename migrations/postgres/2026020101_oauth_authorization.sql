-- OAuth authorization request table for storing pending authorization codes
-- Used in the OAuth-like flow for third-party apps to get scoped tokens
CREATE TABLE oauth_authorization_request (
    code VARCHAR(64) PRIMARY KEY,              -- The authorization code
    entity_id INT NOT NULL,                    -- User who authorized

    -- PKCE (only S256 supported)
    code_challenge VARCHAR(128) NOT NULL,      -- BASE64URL(SHA256(code_verifier))

    -- Request details
    redirect_uri TEXT NOT NULL,
    app_name VARCHAR(255) NOT NULL,
    requested_query_permission_level SMALLINT NOT NULL, -- What the app requested (QueryMode value)
    state VARCHAR(255),                        -- Passed through to redirect

    -- Selected by user during authorization
    database_id INT NOT NULL,                  -- The database user selected or created
    query_permission_level SMALLINT NOT NULL,  -- The permission level user approved (may be <= requested)

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,             -- Code expires after 10 minutes
    used_at TIMESTAMP,                         -- NULL if unused, timestamp when exchanged for token

    FOREIGN KEY(entity_id) REFERENCES entity(id),
    FOREIGN KEY(database_id) REFERENCES database(id)
);

-- Index for faster lookups by code
CREATE INDEX idx_oauth_authorization_request_code ON oauth_authorization_request(code);
