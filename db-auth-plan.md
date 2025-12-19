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
2. App redirects user to ayb authorization page
3. User logs in to ayb (if not already logged in)
4. User sees what the app is requesting (database access, permission level)
5. User picks an existing database or creates a new one
6. User confirms the permission level (read-only or read-write)
7. Ayb creates a scoped token for just that database and permission level
8. Ayb redirects back to the app with the token
9. App stores the token and can now make API calls

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
│      &client_id=todos-app                                           │
│      &redirect_uri=https://todos.example.com/callback               │
│      &scope=database:marcua/todos:read-write                        │
│      &state=random123                                               │
│                                                                     │
│ 3. User authenticates with ayb (if not logged in)                   │
│                                                                     │
│ 4. User sees: "todos-app wants read-write access to marcua/todos"   │
│    User clicks "Authorize"                                          │
│                                                                     │
│ 5. Ayb redirects to:                                                │
│    https://todos.example.com/callback?code=xyz&state=random123      │
│                                                                     │
│ 6. App exchanges code for token:                                    │
│    POST https://ayb.example.com/oauth/token                         │
│    { code: "xyz", client_id: "todos-app", ... }                     │
│                                                                     │
│ 7. Ayb returns scoped token:                                        │
│    { "access_token": "ayb_xxx_yyy", "scope": "..." }                │
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
ALTER TABLE api_token ADD COLUMN permission_level SMALLINT;

-- Add OAuth metadata
ALTER TABLE api_token ADD COLUMN app_name VARCHAR(255);
ALTER TABLE api_token ADD COLUMN app_origin VARCHAR(255);
ALTER TABLE api_token ADD COLUMN created_at TIMESTAMP;
ALTER TABLE api_token ADD COLUMN last_used_at TIMESTAMP;
```

**Notes:**
- `database_id = NULL` means the token works for all databases the user owns/has access to (existing behavior)
- `permission_level = NULL` means full access (existing behavior); otherwise ReadOnly=1, ReadWrite=2
- `app_name` and `app_origin` are for display and validation of OAuth-created tokens
- Existing tokens continue to work unchanged - they just have NULL for the new columns

#### 1.2 Modify Token Validation

Update `retrieve_and_validate_api_token` in `src/server/tokens.rs` to:

1. Check both `api_token` (legacy) and `scoped_api_token` tables
2. Return scope information along with the token
3. Pass scope info through the request pipeline

```rust
pub struct ValidatedToken {
    pub entity_id: i32,
    pub short_token: String,
    pub database_scope: Option<i32>,      // None = all databases
    pub permission_level: PermissionLevel, // ReadOnly or ReadWrite
}

pub enum PermissionLevel {
    ReadOnly,
    ReadWrite,
    Full,  // For legacy tokens
}
```

#### 1.3 Enforce Scopes in Endpoints

Modify permission checks in `src/server/permissions.rs`:

```rust
pub async fn highest_query_access_level(
    authenticated_entity: &InstantiatedEntity,
    database: &InstantiatedDatabase,
    token_scope: &ValidatedToken,  // NEW: Pass token scope
    ayb_db: &web::Data<Box<dyn AybDb>>,
) -> Result<Option<QueryMode>, AybError> {
    // First check if token is scoped to this database
    if let Some(scoped_db_id) = token_scope.database_scope {
        if scoped_db_id != database.id {
            return Ok(None);  // Token can't access this database
        }
    }

    // Then check token's permission cap
    let token_cap = match token_scope.permission_level {
        PermissionLevel::ReadOnly => QueryMode::ReadOnly,
        PermissionLevel::ReadWrite | PermissionLevel::Full => QueryMode::ReadWrite,
    };

    // Get user's actual permission level
    let user_permission = /* existing logic */;

    // Return the more restrictive of the two
    Ok(Some(std::cmp::min(user_permission, token_cap)))
}
```

### Phase 2: Authorization Flow (OAuth-like Endpoints)

#### 2.1 Authorization Request Storage

Create a table to store pending authorization requests:

```sql
CREATE TABLE oauth_authorization_request (
    code VARCHAR(64) PRIMARY KEY,              -- The authorization code
    entity_id INT NOT NULL,                    -- User who authorized

    -- PKCE
    code_challenge VARCHAR(128) NOT NULL,      -- SHA256 hash of verifier
    code_challenge_method VARCHAR(10) NOT NULL, -- "S256" or "plain"

    -- Request details
    redirect_uri TEXT NOT NULL,
    app_name VARCHAR(255),
    requested_scope TEXT NOT NULL,             -- JSON: {"database": "marcua/todos", "permission": "read-write"}
    state VARCHAR(255),                        -- Passed through to redirect

    -- Selected by user
    database_id INT,                           -- The database user selected
    permission_level SMALLINT,                 -- The permission level user approved

    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,             -- Code expires after 10 minutes
    used BOOLEAN NOT NULL DEFAULT FALSE,

    FOREIGN KEY(entity_id) REFERENCES entity(id),
    FOREIGN KEY(database_id) REFERENCES database(id)
);
```

#### 2.2 New API Endpoints

**GET `/oauth/authorize`** - Start authorization flow (redirects to UI)

Query parameters:
- `response_type=code` (required)
- `redirect_uri` - Where to redirect after authorization
- `scope` - What access is requested (format: `database:entity/db:permission` or `database:new:permission`)
- `state` - Opaque value passed through (for CSRF protection)
- `code_challenge` - PKCE challenge
- `code_challenge_method` - Must be "S256"
- `app_name` - Display name for the app (optional, shown to user)

This endpoint validates parameters and redirects to the authorization UI.

**POST `/oauth/token`** - Exchange code for token

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
    "scope": "database:marcua/todos:read-write",
    "ayb_database_url": "https://ayb.example.com/v1/marcua/todos"
}
```

#### 2.3 New UI Endpoints

**GET `/oauth/authorize-ui`** - Authorization consent page

Shows the user:
- What app is requesting access
- Dropdown to select existing database OR create new one
- Permission level selection (read-only / read-write)
- Authorize / Deny buttons

If user is not logged in, redirects to login with a return URL.

**POST `/oauth/authorize-ui`** - Process authorization decision

If user authorizes:
1. Create authorization code
2. Store in `oauth_authorization_request` table
3. Redirect to `redirect_uri?code=xyz&state=...`

If user denies:
1. Redirect to `redirect_uri?error=access_denied&state=...`

### Phase 3: Token Management UI

#### 3.1 Token List Page

Add `GET /{entity}/tokens` UI endpoint showing:
- All active tokens for the user
- For each token: app name, database scope, permission level, created date, last used
- "Revoke" button for each token

#### 3.2 Token Revocation API

Add `DELETE /v1/tokens/{short_token}` API endpoint to revoke a token.

---

## Dynamic Server Discovery

One unique requirement is that apps should work with any ayb server, even ones the app developer doesn't know about. Here's how this works:

### Option A: User Provides Server URL

The simplest approach:

1. App asks user: "Enter your ayb server URL" (with a default like `ayb.io`)
2. App constructs authorization URL: `{server_url}/oauth/authorize?...`
3. After authorization, app stores both the server URL and token

```javascript
// In the todos app
const aybServer = localStorage.getItem('ayb_server')
    || prompt('Enter your ayb server URL:', 'https://ayb.io');
const authUrl = `${aybServer}/oauth/authorize?...`;
window.location.href = authUrl;
```

### Option B: Server Discovery via Well-Known URL

Apps can discover OAuth endpoints via a well-known configuration:

**GET `/.well-known/ayb-configuration`**
```json
{
    "authorization_endpoint": "/oauth/authorize",
    "token_endpoint": "/oauth/token",
    "supported_scopes": ["database:read-only", "database:read-write"],
    "code_challenge_methods_supported": ["S256"]
}
```

This is optional but helpful for apps that want to validate they're talking to a real ayb server.

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
    constructor(options = {}) {
        this.serverUrl = options.serverUrl || 'https://ayb.io';
        this.appName = options.appName || 'Unknown App';
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
            scope: options.scope || 'database:new:read-write',
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
        const response = await fetch(`${this.serverUrl}/oauth/token`, {
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
            databaseUrl: tokenData.ayb_database_url,
            scope: tokenData.scope
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
    serverUrl: localStorage.getItem('ayb_server') || 'https://ayb.io',
    appName: 'Todos App'
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
    ayb.authorize({
        scope: 'database:new:read-write'  // Create new DB with read-write
    });
}
```

---

## Scope Format

Scopes define what access the app is requesting:

| Scope | Meaning |
|-------|---------|
| `database:entity/dbname:read-only` | Read-only access to specific database |
| `database:entity/dbname:read-write` | Read-write access to specific database |
| `database:new:read-only` | Create new database with read-only access |
| `database:new:read-write` | Create new database with read-write access |
| `database:pick:read-only` | Let user pick existing database, read-only |
| `database:pick:read-write` | Let user pick existing database, read-write |

The authorization UI interprets these scopes:
- `entity/dbname` - Pre-selected database (user can change)
- `new` - Show "create new database" form
- `pick` - Show database picker

---

## Security Considerations

### 1. Redirect URI Validation

When an app initiates authorization:
- Store the `redirect_uri` with the authorization request
- On token exchange, verify the `redirect_uri` matches exactly
- Only allow `https://` URIs (except `http://localhost` for development)

### 2. Code Expiration

Authorization codes should:
- Expire after 10 minutes
- Be single-use (mark as `used` after exchange)
- Be cryptographically random (64+ characters)

### 3. PKCE Enforcement

For public clients (frontend apps):
- Always require `code_challenge` in authorization request
- Reject token requests without valid `code_verifier`
- Only support `S256` method (not `plain`)

### 4. Token Scoping

Scoped tokens should:
- Never exceed the permission level the user has
- Be revocable by the user at any time
- Show clear audit trail (which app, when created, last used)

### 5. CORS Configuration

The `/oauth/token` endpoint needs CORS headers to allow:
- Requests from any origin (the callback page)
- POST method
- Content-Type header

Current ayb CORS config already allows this.

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

## Example: Updated Todos App Flow

Here's how the todos app would work with this system:

```
┌─────────────────────────────────────────────────────────────────────┐
│ User visits https://marcua.net/minitools/todos/                     │
│                                                                     │
│ 1. App checks localStorage - no token found                         │
│                                                                     │
│ 2. App shows: "Connect to your ayb database"                        │
│    [ayb server: https://ayb.io    ] [Connect]                       │
│                                                                     │
│ 3. User clicks Connect, app redirects to:                           │
│    https://ayb.io/oauth/authorize?                                  │
│      response_type=code                                             │
│      &redirect_uri=https://marcua.net/minitools/todos/              │
│      &scope=database:new:read-write                                 │
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
│    │ "Todos" wants to access your ayb database    │                 │
│    │                                              │                 │
│    │ Database: [Create new: todos     ▼]          │                 │
│    │           (or select existing)               │                 │
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
│    POST https://ayb.io/oauth/token                                  │
│    { code: "xyz", code_verifier: "dBjftJeZ...", ... }               │
│                                                                     │
│ 8. App receives:                                                    │
│    {                                                                │
│      "access_token": "ayb_xxx_yyy",                                 │
│      "ayb_database_url": "https://ayb.io/v1/marcua/todos"           │
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

2. **Rate limiting**: Should we add rate limits per token to prevent abuse?

3. **Token rotation**: Should apps be able to request a new token before the old one expires?

4. **Organization support**: How do org-owned databases work with this flow?

5. **Multiple databases**: Should one authorization grant access to multiple databases?

6. **Webhook notifications**: Should ayb notify apps when tokens are revoked?

---

## Summary

This plan adds database-scoped, permission-limited tokens to ayb, with an OAuth 2.0 + PKCE authorization flow that works for frontend-only applications. The key features are:

1. **Scoped tokens**: Tokens can be limited to specific databases and permission levels
2. **OAuth flow**: Standard authorization code flow with PKCE for security
3. **Frontend-friendly**: No backend required for apps
4. **Server-agnostic**: Apps can work with any ayb server the user chooses
5. **User control**: Users decide exactly what access to grant and can revoke anytime

The implementation can be phased, starting with scoped tokens and basic OAuth, then adding UX improvements and a client library.
