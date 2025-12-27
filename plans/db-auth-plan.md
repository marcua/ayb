# Database Authorization Plan for Third-Party Apps

## Problem Statement

Today, if a third-party app (like the [todos app](https://marcua.net/minitools/todos/)) wants to connect to an ayb database, users must:

1. Manually create an ayb account and database
2. Copy their API token from ayb
3. Paste the token into the app's configuration form
4. Also provide the database URL

This is not ideal because:
- **Tokens are overpowered**: Current tokens grant full access to the user's entire ayb account, not just one database
- **No permission scoping**: Apps can't request just read-only access; they get whatever the user's token allows
- **Manual and error-prone**: Copy-pasting tokens is tedious and insecure
- **No discoverability**: Apps can't help users find or create databases

### The Ideal Flow

1. App detects it doesn't have credentials stored
2. App redirects user to ayb authorization page, specifying desired permission level (read-only or read-write)
3. User logs in to ayb (if not already logged in)
4. User sees what permission level the app is requesting
5. User picks an existing database or creates a new one
6. User confirms the permission level (can downgrade from app's request)
7. Ayb creates a scoped token for just that database and permission level
8. Ayb redirects back to the app with an authorization code
9. App exchanges code for token and can now make API calls

### Codebase Context

**Endpoint Organization:**
- API endpoints live in `src/server/api_endpoints/` and are mounted under `/v1/` with bearer token auth
- UI endpoints live in `src/server/ui_endpoints/` and use cookie-based auth
- Routes are configured in `src/server/server_runner.rs`

**Testing Strategy:**
- End-to-end tests in `tests/e2e_tests/` test API via CLI commands
- Browser e2e tests in `tests/browser_e2e_tests/` test UI flows
- Test utilities in `tests/utils/` provide helper functions

---

## Background: OAuth 2.0 and PKCE

### What is OAuth 2.0?

OAuth 2.0 is a standard authorization framework that allows third-party applications to obtain limited access to a web service on behalf of a user, without the user sharing their password. Key concepts:

- **Resource Owner**: The user who owns the data
- **Client**: The third-party application requesting access
- **Authorization Server**: The server that authenticates the user and issues tokens (ayb in our case)
- **Resource Server**: The server hosting the protected data (also ayb)

### Authorization Code Flow

The most secure OAuth flow works like this:

```
┌─────────────────────────────────────────────────────────────────────┐
│ 1. User clicks "Connect to ayb" in the app                          │
│                                                                     │
│ 2. App redirects to:                                                │
│    https://ayb.example.com/oauth/authorize?                         │
│      response_type=code                                             │
│      &redirect_uri=https://todos.example.com/callback               │
│      &scope=read-write                                              │
│      &app_name=Todos                                                │
│      &state=random123                                               │
│                                                                     │
│ 3. User authenticates with ayb (if not logged in)                   │
│                                                                     │
│ 4. User sees: "Todos wants read-write access"                       │
│    User picks database: [marcua/todos ▼]                            │
│    User clicks "Authorize"                                          │
│                                                                     │
│ 5. Ayb redirects to:                                                │
│    https://todos.example.com/callback?code=xyz&state=random123      │
│                                                                     │
│ 6. App exchanges code for token:                                    │
│    POST https://ayb.example.com/oauth/token                         │
│    { code: "xyz", code_verifier: "...", ... }                       │
│                                                                     │
│ 7. Ayb returns scoped token:                                        │
│    { "access_token": "ayb_xxx_yyy", "database": "marcua/todos" }    │
└─────────────────────────────────────────────────────────────────────┘
```

### The Problem with Traditional OAuth for SPAs

Traditional OAuth assumes the client has a backend server to:
1. Store a **client secret** (a password only the app knows)
2. Exchange the authorization code for a token server-side

But frontend-only apps (like the todos app) have no backend and can't securely store secrets.

### PKCE: The Solution for Frontend Apps

**PKCE** (Proof Key for Code Exchange, pronounced "pixy") solves this by replacing the client secret with a dynamically-generated proof:

```
┌─────────────────────────────────────────────────────────────────────┐
│ PKCE Flow:                                                          │
│                                                                     │
│ 1. App generates a random "code_verifier" (43-128 characters)       │
│    Example: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"           │
│                                                                     │
│ 2. App hashes it to create "code_challenge"                         │
│    code_challenge = BASE64URL(SHA256(code_verifier))                │
│    Example: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"           │
│                                                                     │
│ 3. App sends code_challenge in authorization request                │
│    (but NOT the code_verifier)                                      │
│                                                                     │
│ 4. User authorizes, ayb stores the code_challenge with the code     │
│                                                                     │
│ 5. App exchanges code for token, sending the code_verifier          │
│                                                                     │
│ 6. Ayb verifies: SHA256(code_verifier) matches stored code_challenge│
│    If it matches, the request is legitimate                         │
└─────────────────────────────────────────────────────────────────────┘
```

This works because:
- An attacker who intercepts the authorization code doesn't know the `code_verifier`
- The `code_verifier` was only stored in the original browser tab that started the flow
- Without the `code_verifier`, the attacker can't exchange the code for a token

---

## Design for ayb

### Goals

1. **Frontend-only apps**: Apps like todos should work without any backend
2. **Database-scoped tokens**: Tokens should only grant access to specific databases
3. **Permission-scoped tokens**: Tokens should specify read-only vs read-write
4. **Dynamic server discovery**: Apps should be able to work with any ayb server, even ones the app developer doesn't know about
5. **User-controlled**: Users decide which database and what permissions to grant

### Non-Goals (for initial implementation)

- Full OAuth 2.0 compliance (we'll implement what we need)
- Refresh tokens (can be added later)
- Client registration (apps are implicitly "public clients")

---

## Implementation Plan

### Phase 1: Scoped Tokens (Database Layer Changes)

#### 1.1 Database Migration: Add Scope Columns to `api_token`

Extend the existing `api_token` table with optional scope columns:

```sql
-- Add scope fields (all nullable for backward compatibility)
ALTER TABLE api_token ADD COLUMN database_id INT REFERENCES database(id);
ALTER TABLE api_token ADD COLUMN query_permission_level SMALLINT;

-- Add OAuth metadata
ALTER TABLE api_token ADD COLUMN granted_by INT REFERENCES entity(id);
ALTER TABLE api_token ADD COLUMN app_name VARCHAR(255);
ALTER TABLE api_token ADD COLUMN app_origin_url VARCHAR(255);
ALTER TABLE api_token ADD COLUMN created_at TIMESTAMP;
ALTER TABLE api_token ADD COLUMN expires_at TIMESTAMP;
ALTER TABLE api_token ADD COLUMN revoked_at TIMESTAMP;
```

**Notes:**
- `database_id = NULL` means the token works for all databases the user owns/has access to (existing behavior)
- `query_permission_level` uses the same values as `QueryMode`: ReadOnly=0, ReadWrite=1, NULL=no cap
- `granted_by` tracks which entity authorized this token (for shared databases, may differ from `entity_id`)
- `app_name` is the display name shown to users (e.g., "Todos")
- `app_origin_url` is the full URL origin that created the token (e.g., "https://todos.example.com")
- `expires_at = NULL` means the token never expires (default); otherwise token becomes invalid after this time
- `revoked_at = NULL` means the token is active; set to timestamp when user revokes (retained for audit trail)
- Existing tokens continue to work unchanged - they just have NULL for the new columns

**Token scope restrictions:**
- A database-scoped token (`database_id != NULL`) can only access that specific database
- A database-scoped token cannot be used to list databases, create databases, or manage other tokens
- Only unscoped tokens (`database_id = NULL`) can perform account-level operations

#### 1.2 Modify Token Validation

Update `retrieve_and_validate_api_token` in `src/server/tokens.rs` to:

1. Check token is not revoked (`revoked_at IS NULL`)
2. Check token expiration (if `expires_at` is set and in the past, reject)
3. Return scope information along with the token
4. Pass scope info through the request pipeline

```rust
pub struct ValidatedToken {
    pub entity_id: i32,
    pub short_token: String,
    pub database_id: Option<i32>,              // None = all databases
    pub query_permission_level: Option<QueryMode>, // None = no cap (use user's permission)
}
```

Note: We reuse the existing `QueryMode` enum (ReadOnly=0, ReadWrite=1) rather than creating a new one.

#### 1.3 Enforce Scopes in Endpoints

Modify permission checks in `src/server/permissions.rs`:

```rust
pub async fn highest_query_access_level(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
    token: &ValidatedToken,  // NEW: Pass validated token
    ayb_db: &web::Data<Box<dyn AybDb>>,
) -> Result<Option<QueryMode>, AybError> {
    // First check if token is scoped to a specific database
    if let Some(token_db_id) = token.database_id {
        if token_db_id != database.id {
            return Ok(None);  // Token can't access this database
        }
    }

    // Get user's actual permission level (existing logic)
    let user_permission: Option<QueryMode> = /* existing logic */;

    // Return the more restrictive of token and user permissions
    // Examples:
    //   - User is owner (ReadWrite), token has ReadOnly → ReadOnly
    //   - User has ReadOnly, token has ReadWrite → ReadOnly
    //   - User is owner (ReadWrite), token has no cap (None) → ReadWrite
    match (user_permission, token.query_permission_level) {
        (None, _) => Ok(None),  // User has no access
        (Some(user), None) => Ok(Some(user)),  // No token cap, use user permission
        (Some(QueryMode::ReadOnly), Some(_)) => Ok(Some(QueryMode::ReadOnly)),  // User is read-only, can't upgrade
        (Some(QueryMode::ReadWrite), Some(QueryMode::ReadOnly)) => Ok(Some(QueryMode::ReadOnly)),  // Token caps to read-only
        (Some(QueryMode::ReadWrite), Some(QueryMode::ReadWrite)) => Ok(Some(QueryMode::ReadWrite)),  // Both allow write
    }
}
```

The key insight: the effective permission is the more restrictive of user and token permissions. We use explicit pattern matching rather than relying on the numeric representation of `QueryMode`. A read-only token can never write, even if the user is an owner. And a read-write token can't grant more access than the user actually has.

### Phase 2: Authorization Flow (OAuth-like Endpoints)

#### 2.1 Authorization Request Storage

Create a table to store pending authorization requests:

```sql
CREATE TABLE oauth_authorization_request (
    code VARCHAR(64) PRIMARY KEY,              -- The authorization code
    entity_id INT NOT NULL,                    -- User who authorized

    -- PKCE (only S256 supported)
    code_challenge VARCHAR(128) NOT NULL,      -- BASE64URL(SHA256(code_verifier))

    -- Request details
    redirect_uri TEXT NOT NULL,
    app_name VARCHAR(255),
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
```

**Notes:**
- Only S256 (SHA-256) is supported for `code_challenge` - the `plain` method is insecure and not implemented
- `used_at` is NULL until the code is exchanged for a token, then set to the exchange timestamp
- Codes with non-NULL `used_at` or past `expires_at` are invalid

#### 2.2 New API Endpoint

Add to `src/server/api_endpoints/`:

**POST `/v1/oauth/token`** - Exchange code for token

Request body (form-encoded or JSON):
```json
{
    "grant_type": "authorization_code",
    "code": "xyz123...",
    "redirect_uri": "https://todos.example.com/callback",
    "code_verifier": "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
}
```

Response:
```json
{
    "access_token": "ayb_abc123_def456",
    "token_type": "Bearer",
    "database": "marcua/todos",
    "query_permission_level": "read-write",
    "database_url": "https://ayb.example.com/v1/marcua/todos"
}
```

#### 2.3 New UI Endpoints

These endpoints live in `src/server/ui_endpoints/` (cookie-based auth).

**GET `/oauth/authorize`** - Authorization consent page

Query parameters:
- `response_type=code` (required, must be "code") - OAuth 2.0 protocol requirement for future grant type extensibility
- `redirect_uri` (required) - Where to redirect after authorization
- `scope` (required) - Permission level requested: `read-only` or `read-write`
- `state` (required) - Opaque value passed through (for CSRF protection)
- `code_challenge` (required) - PKCE challenge: BASE64URL(SHA256(code_verifier))
- `code_challenge_method` (required) - Must be "S256"; OAuth 2.0 protocol requirement for future method extensibility
- `app_name` (required) - Display name for the app (shown to user)

If user is not logged in, redirects to login with a return URL that preserves all parameters.

Shows the user:
- What app is requesting access (app_name)
- What permission level is requested
- Dropdown to select existing database OR create new one. Creation can reuse the new database creation interface on the database list page in the UI.
- Permission level selector (can downgrade from requested, not upgrade)
- Authorize / Deny buttons

**POST `/oauth/authorize`** - Process authorization decision

If user authorizes:
1. If creating new database, create it first
2. Generate authorization code
3. Store in `oauth_authorization_request` table
4. Redirect to `redirect_uri?code=xyz&state=...`

If user denies:
1. Redirect to `redirect_uri?error=access_denied&state=...`

### Phase 3: Token Management

**Important:** These endpoints require an unscoped token (one with `database_id = NULL`). A database-scoped token cannot list or manage other tokens, as that would allow an app to escalate its permissions.

#### 3.1 API Endpoints

Add to `src/server/api_endpoints/`:

**GET `/v1/tokens`** - List all tokens for authenticated entity
```json
{
    "tokens": [
        {
            "short_token": "abc123...",
            "database": "marcua/todos",        // null for unscoped tokens
            "query_permission_level": "read-write",
            "app_name": "Todos",
            "created_at": "2024-01-15T10:30:00Z",
            "expires_at": null,
            "revoked_at": null                 // null if active, timestamp if revoked
        }
    ]
}
```

**DELETE `/v1/tokens/{short_token}`** - Revoke a token

Sets `revoked_at` timestamp rather than deleting the row, for audit trail purposes. Revoked tokens are excluded from the list by default but retained in the database.

#### 3.2 CLI Commands

Add to `src/client/`:

```bash
ayb client list_tokens                    # List all tokens
ayb client revoke_token <short_token>     # Revoke a token
```

#### 3.3 UI Page

Add to `src/server/ui_endpoints/`:

**GET `/{entity}/tokens`** - Token management page

Access via username dropdown in the header (click username → dropdown with "Tokens" and "Log out" options).

Shows:
- All active tokens for the user
- For each token: app name, database scope, permission level, created date, expiration
- "Revoke" button for each token

---

## Dynamic Server Discovery

One unique requirement is that apps should work with any ayb server, even ones the app developer doesn't know about. Here's how this works:

### Server Selection UI

Apps should provide a dropdown for server selection:

```
┌────────────────────────────────────────┐
│ Connect to your database               │
│                                        │
│ Server: [The Data (https://thedata.zone) ▼]
│         ├─ The Data (https://thedata.zone)
│         ├─ Other...
│         └─────────────────────────────
│                                        │
│ [Connect]                              │
└────────────────────────────────────────┘
```

If "Other..." is selected, show a text input for custom server URL.

After authorization, the app stores both the server URL and token together.

### Security Consideration: Trusting Unknown Servers

Since apps can connect to any ayb server:

1. **Apps should warn users** when connecting to non-default servers
2. **Apps should remember** which server a token came from
3. **Tokens are server-specific** - a token from server A won't work on server B

This is actually not different from traditional OAuth - users already trust their identity provider. Here, users are trusting their data provider.

---

## Client-Side Library (Optional)

To make integration easier, ayb could provide a small JavaScript library:

```javascript
// ayb-oauth.js

class AybOAuth {
    /**
     * @param {Object} options
     * @param {string} options.appName - Required. Display name shown to users during authorization
     * @param {string} options.queryPermissionLevel - Required. 'read-only' or 'read-write'
     * @param {string} [options.serverUrl] - Optional. Defaults to 'https://thedata.zone'
     * @param {string} [options.storageKey] - Optional. localStorage key. Defaults to 'ayb_auth'
     */
    constructor(options) {
        if (!options.appName) throw new Error('appName is required');
        if (!options.queryPermissionLevel) throw new Error('queryPermissionLevel is required');
        if (!['read-only', 'read-write'].includes(options.queryPermissionLevel)) {
            throw new Error('queryPermissionLevel must be "read-only" or "read-write"');
        }

        this.serverUrl = options.serverUrl || 'https://thedata.zone';
        this.appName = options.appName;
        this.queryPermissionLevel = options.queryPermissionLevel;
        this.storageKey = options.storageKey || 'ayb_auth';
    }

    // Check if we have valid credentials
    isAuthenticated() {
        return !!this.getToken();
    }

    getToken() {
        const auth = localStorage.getItem(this.storageKey);
        return auth ? JSON.parse(auth) : null;
    }

    // Start the authorization flow
    async authorize(options = {}) {
        const codeVerifier = this.generateCodeVerifier();
        const codeChallenge = await this.sha256(codeVerifier);
        const state = this.generateState();

        // Store verifier and state for later
        sessionStorage.setItem('ayb_pkce_verifier', codeVerifier);
        sessionStorage.setItem('ayb_oauth_state', state);

        const params = new URLSearchParams({
            response_type: 'code',
            redirect_uri: window.location.origin + (options.callbackPath || '/'),
            scope: this.queryPermissionLevel,
            state: state,
            code_challenge: codeChallenge,
            code_challenge_method: 'S256',
            app_name: this.appName
        });

        window.location.href = `${this.serverUrl}/oauth/authorize?${params}`;
    }

    // Handle the callback (call this on page load)
    async handleCallback() {
        const params = new URLSearchParams(window.location.search);
        const code = params.get('code');
        const state = params.get('state');
        const error = params.get('error');

        if (error) {
            throw new Error(`Authorization failed: ${error}`);
        }

        if (!code) return false; // Not a callback

        // Verify state
        const savedState = sessionStorage.getItem('ayb_oauth_state');
        if (state !== savedState) {
            throw new Error('State mismatch - possible CSRF attack');
        }

        // Exchange code for token
        const codeVerifier = sessionStorage.getItem('ayb_pkce_verifier');
        const response = await fetch(`${this.serverUrl}/v1/oauth/token`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                grant_type: 'authorization_code',
                code: code,
                redirect_uri: window.location.origin + window.location.pathname,
                code_verifier: codeVerifier
            })
        });

        if (!response.ok) {
            throw new Error('Token exchange failed');
        }

        const tokenData = await response.json();

        // Store the token
        localStorage.setItem(this.storageKey, JSON.stringify({
            serverUrl: this.serverUrl,
            token: tokenData.access_token,
            database: tokenData.database,
            databaseUrl: tokenData.database_url,
            queryPermissionLevel: tokenData.query_permission_level
        }));

        // Clean up URL
        window.history.replaceState({}, '', window.location.pathname);

        // Clean up session storage
        sessionStorage.removeItem('ayb_pkce_verifier');
        sessionStorage.removeItem('ayb_oauth_state');

        return true;
    }

    // Make an authenticated API request
    async query(sql) {
        const auth = this.getToken();
        if (!auth) throw new Error('Not authenticated');

        const response = await fetch(`${auth.databaseUrl}/query`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${auth.token}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ query: sql })
        });

        return response.json();
    }

    // Helper methods
    generateCodeVerifier() {
        const array = new Uint8Array(32);
        crypto.getRandomValues(array);
        return this.base64UrlEncode(array);
    }

    generateState() {
        const array = new Uint8Array(16);
        crypto.getRandomValues(array);
        return this.base64UrlEncode(array);
    }

    async sha256(str) {
        const encoder = new TextEncoder();
        const data = encoder.encode(str);
        const hash = await crypto.subtle.digest('SHA-256', data);
        return this.base64UrlEncode(new Uint8Array(hash));
    }

    base64UrlEncode(array) {
        return btoa(String.fromCharCode(...array))
            .replace(/\+/g, '-')
            .replace(/\//g, '_')
            .replace(/=+$/, '');
    }

    // Disconnect / logout
    disconnect() {
        localStorage.removeItem(this.storageKey);
    }
}
```

Usage in the todos app:

```javascript
const ayb = new AybOAuth({
    appName: 'Todos',
    queryPermissionLevel: 'read-write',
    serverUrl: localStorage.getItem('ayb_server') || 'https://thedata.zone'
});

// On page load
async function init() {
    // Check if this is a callback from OAuth
    if (await ayb.handleCallback()) {
        console.log('Successfully connected to ayb!');
    }

    if (ayb.isAuthenticated()) {
        // Load todos from ayb
        const result = await ayb.query('SELECT * FROM todos ORDER BY created_at');
        renderTodos(result.rows);
    } else {
        // Show connect button
        showConnectUI();
    }
}

function connect() {
    ayb.authorize();  // User will pick/create database on ayb
}
```

---

## Scope Format

Scopes are simple permission levels:

| Scope | Meaning |
|-------|---------|
| `read-only` | App requests read-only access |
| `read-write` | App requests read-write access |

The app only specifies the permission level. The user chooses which database to grant access to (existing or new) during the authorization flow.

The user can also downgrade the permission (e.g., grant read-only when app requested read-write), but cannot upgrade it.

---

## Security Considerations

### 1. Redirect URI Validation

When an app initiates authorization:
- Store the `redirect_uri` with the authorization request
- On token exchange, verify the `redirect_uri` matches exactly
- Only allow `https://` URIs (except `http://localhost` for development)

**Implementation:** Validate in `/oauth/authorize` (UI endpoint) before storing, and verify match in `/v1/oauth/token`.

### 2. Code Expiration

Authorization codes should:
- Expire after 10 minutes
- Be single-use (set `used_at` timestamp after exchange)
- Be cryptographically random (64+ characters using `OsRng`)

**Implementation:** Check `expires_at` and `used_at IS NULL` in `/v1/oauth/token`.

### 3. PKCE Enforcement

For public clients (frontend apps):
- Always require `code_challenge` in authorization request
- Reject token requests without valid `code_verifier`
- Only support `S256` method (SHA-256, base64url-encoded)

**Implementation:**
- In `/oauth/authorize`: require `code_challenge` and `code_challenge_method=S256`
- In `/v1/oauth/token`: compute `BASE64URL(SHA256(code_verifier))` and compare to stored `code_challenge`

### 4. Token Scoping

Scoped tokens should:
- Never exceed the permission level the user has (enforced by `min()` logic)
- Be revocable by the user at any time via `/v1/tokens/{short_token}` DELETE
- Show clear audit trail (app_name, created_at, expires_at)

**Implementation:** All permission checks go through `highest_query_access_level()` which applies the min.

### 5. CORS Configuration

The `/v1/oauth/token` endpoint needs CORS headers to allow:
- Requests from any origin (the callback page)
- POST method
- Content-Type header

**Implementation:** Current ayb CORS config (`cors.origin = "*"`) already allows this.

### 6. State Parameter

The `state` parameter prevents CSRF attacks:
- App generates random state, stores in sessionStorage
- App includes state in authorization request
- ayb passes state through unchanged in redirect
- App verifies returned state matches stored state

**Implementation:** ayb just passes `state` through; the client library handles generation and verification.

### 7. Security Review Summary

A security engineer reviewing this implementation should verify:

**Authentication & Authorization:**
- [ ] PKCE verifier/challenge validation uses constant-time comparison
- [ ] Authorization codes are generated with cryptographically secure randomness (`OsRng`)
- [ ] Token hashes use a secure algorithm (SHA-256 with proper salting)
- [ ] Database-scoped tokens cannot access other databases or account-level endpoints
- [ ] Permission intersection logic correctly prevents privilege escalation

**Input Validation:**
- [ ] `redirect_uri` is validated against allowlist (https only, except localhost)
- [ ] `redirect_uri` comparison is exact (no partial matching that could be bypassed)
- [ ] All OAuth parameters are properly sanitized before storage/use
- [ ] SQL injection prevented via parameterized queries (already standard in ayb)
- [ ] `/oauth/` is in the restricted registration username list (prevents entity `oauth` from claiming the path)

**Session Security:**
- [ ] Authorization codes expire and are single-use
- [ ] Tokens can be revoked and revocation is checked on every request
- [ ] `state` parameter is passed through opaquely (client-side CSRF protection)

**Information Disclosure:**
- [ ] Error messages don't leak sensitive information about valid/invalid tokens
- [ ] Token list endpoint only returns tokens owned by the authenticated entity
- [ ] Audit trail (`revoked_at`, `created_at`) doesn't expose sensitive timing info

---

## Implementation Order

### Step 1: Scoped Tokens (Foundation)
1. Add scope columns to `api_token` table
2. Modify token validation to return scope info
3. Enforce scopes in permission checks
4. Add token management API (list, revoke)

### Step 2: Basic OAuth Flow
1. Add `oauth_authorization_request` table migration
2. Implement `/oauth/authorize` redirect endpoint
3. Build authorization consent UI
4. Implement `/oauth/token` exchange endpoint
5. Add PKCE validation

### Step 3: Enhanced UX
1. Add token management UI page
2. Build database picker component
3. Add "create new database" option in auth flow
4. Implement the client-side library

### Step 4: Polish
1. Add well-known configuration endpoint
2. Improve error messages and UX
3. Add analytics/audit logging
4. Document the OAuth flow for app developers

---

## Testing Strategy

### E2E Tests (in `tests/e2e_tests/`)

Add `oauth_tests.rs` with tests for:

1. **Token scoping**
   - Token scoped to database A cannot query database B
   - Token with read-only cannot write even if user has write access
   - Token with write access limited to user's actual permission level
   - Token with `expires_at` in past is rejected
   - Database-scoped token cannot be used to list or create databases

2. **OAuth flow**
   - Valid authorization request redirects to consent UI
   - Invalid parameters (missing `code_challenge`, bad `redirect_uri`) return errors
   - Token exchange with valid code and verifier succeeds
   - Token exchange with invalid verifier fails
   - Token exchange with expired code fails
   - Token exchange with already-used code fails

3. **Token management**
   - `GET /v1/tokens` lists user's tokens
   - `DELETE /v1/tokens/{short_token}` revokes token
   - Revoked token cannot be used for queries

### Browser E2E Tests (in `tests/browser_e2e_tests/`)

Add `oauth_tests.rs` with tests for:

1. **Authorization consent UI**
   - Shows app name and requested permission
   - Database dropdown shows user's databases
   - Can create new database from authorization flow
   - Authorize redirects with code
   - Deny redirects with error

2. **Token management UI**
   - Token list page shows all tokens
   - Revoke button removes token

### Test Utilities (in `tests/utils/`)

Add helpers:
- `oauth_authorize(config, params)` - Start OAuth flow
- `oauth_token_exchange(config, code, verifier)` - Exchange code for token
- `create_scoped_token(config, entity, database, permission)` - Create token directly for testing

---

## Example: Updated Todos App Flow

Here's how the todos app would work with this system:

```
┌─────────────────────────────────────────────────────────────────────┐
│ User visits https://marcua.net/minitools/todos/                     │
│                                                                     │
│ 1. App checks localStorage - no token found                         │
│                                                                     │
│ 2. App shows: "Connect to your database"                            │
│    Server: [The Data (https://thedata.zone) ▼]                      │
│    [Connect]                                                        │
│                                                                     │
│ 3. User clicks Connect, app redirects to:                           │
│    https://thedata.zone/oauth/authorize?                            │
│      response_type=code                                             │
│      &redirect_uri=https://marcua.net/minitools/todos/              │
│      &scope=read-write                                              │
│      &app_name=Todos                                                │
│      &code_challenge=E9Melhoa2OwvFrE...                             │
│      &code_challenge_method=S256                                    │
│      &state=abc123                                                  │
│                                                                     │
│ 4. User is not logged in, ayb shows login page                      │
│    User logs in via email                                           │
│                                                                     │
│ 5. ayb shows authorization page:                                    │
│    ┌──────────────────────────────────────────────┐                 │
│    │ "Todos" wants read-write access              │                 │
│    │                                              │                 │
│    │ Database: [marcua/todos           ▼]         │                 │
│    │           ├─ marcua/todos                    │                 │
│    │           ├─ marcua/other-db                 │                 │
│    │           └─ Create new database...          │                 │
│    │                                              │                 │
│    │ Permission: ○ Read only                      │                 │
│    │             ● Read and write                 │                 │
│    │                                              │                 │
│    │ [Deny]  [Authorize]                          │                 │
│    └──────────────────────────────────────────────┘                 │
│                                                                     │
│ 6. User clicks Authorize, ayb redirects to:                         │
│    https://marcua.net/minitools/todos/?code=xyz&state=abc123        │
│                                                                     │
│ 7. App exchanges code for token:                                    │
│    POST https://thedata.zone/v1/oauth/token                         │
│    { code: "xyz", code_verifier: "dBjftJeZ...", ... }               │
│                                                                     │
│ 8. App receives:                                                    │
│    {                                                                │
│      "access_token": "ayb_xxx_yyy",                                 │
│      "database": "marcua/todos",                                    │
│      "database_url": "https://thedata.zone/v1/marcua/todos"         │
│    }                                                                │
│                                                                     │
│ 9. App stores token in localStorage, ready to use!                  │
│    Now makes queries to the database using the scoped token         │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Comparison to Full OAuth 2.0

| Feature | Full OAuth 2.0 | This Proposal |
|---------|---------------|---------------|
| Client registration | Required | Not needed (public clients only) |
| Client secrets | Required for confidential clients | Not used |
| PKCE | Optional | Required |
| Refresh tokens | Common | Not in v1 (can add later) |
| Scopes | Generic | Database-specific |
| Token introspection | Standard endpoint | Not in v1 |
| Token revocation | Standard endpoint | Simplified version |

This proposal is a simplified, purpose-built OAuth implementation focused on ayb's specific needs rather than full compliance with the OAuth 2.0 specification.

---

## Open Questions

1. **Token expiration**: Should scoped tokens expire? If so, how long? (Current tokens don't expire)
   - Proposal: Add optional `expires_at` column (NULL = never expires). Alert users on token list page when tokens are near expiration.

---

## Future Work

Items explicitly deferred from this implementation:

1. **Rate limiting**: Add rate limits per token to prevent abuse
2. **Expired token cleanup**: Background job to delete tokens past `expires_at`
3. **Refresh tokens**: Allow apps to get new tokens without re-authorization
4. **Organization support**: Allow org-owned databases in the authorization flow

---

## Implementation Status

### Completed: Scoped Tokens (Phase 1 & 3)

The following has been implemented:

- [x] **Database Migration** (Phase 1.1): Added scope columns to `api_token` table
  - `database_id`, `query_permission_level`, `app_name`, `created_at`, `expires_at`, `revoked_at`

- [x] **Token Validation** (Phase 1.2): Updated `retrieve_and_validate_api_token` to:
  - Check token status (revoked tokens are rejected)
  - Check token expiration (expired tokens are rejected)
  - Pass token info through request pipeline

- [x] **Scope Enforcement** (Phase 1.3): Modified permission checks in `permissions.rs`:
  - `highest_query_access_level_with_token()` enforces database scoping and permission caps
  - Token permission is intersected with user permission (most restrictive wins)

- [x] **Token Management API** (Phase 3.1):
  - `GET /v1/tokens` - List all active tokens for authenticated entity
  - `DELETE /v1/tokens/{short_token}` - Revoke a token

- [x] **CLI Commands** (Phase 3.2):
  - `ayb client list_tokens` - List all tokens
  - `ayb client revoke_token <short_token>` - Revoke a token

- [x] **Token Management UI** (Phase 3.3):
  - `GET /{entity}/tokens` - Token management page
  - Shows tokens with scope, permissions, app name, created/expires dates
  - Revoke button for each token

- [x] **Tests**: Added token tests to e2e test suite

### Pending: OAuth Flow (Phase 2)

The OAuth authorization flow has not been implemented yet:

- [ ] `oauth_authorization_request` table migration
- [ ] `/oauth/authorize` redirect endpoint
- [ ] Authorization consent UI
- [ ] `/v1/oauth/token` exchange endpoint
- [ ] PKCE validation
- [ ] Client-side library
