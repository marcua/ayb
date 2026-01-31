/**
 * ayb.js - Client library for building apps on ayb
 *
 * Provides two classes:
 *
 *   AybClient - Base client for manual token auth (URL + token)
 *     const db = new AybClient({ appId: 'my-app' });
 *     db.saveConfig('https://host/entity/database', 'token');
 *     await db.connect(migrations);
 *     const rows = await db.queryObjects('SELECT * FROM todos');
 *
 *   AybOAuth - OAuth authorization flow (extends AybClient)
 *     const ayb = new AybOAuth({ appName: 'My App', queryPermissionLevel: 'read-write' });
 *     if (await ayb.handleCallback()) { ... }
 *     if (!ayb.isAuthenticated()) { ayb.authorize(); }
 *     const rows = await ayb.queryObjects('SELECT * FROM todos');
 */

class AybClient {
    /**
     * @param {Object} [options]
     * @param {string} options.appId - Application identifier, used to scope
     *   localStorage keys and migration state. Required.
     * @param {string} [options.storageKey] - localStorage key prefix.
     *   Defaults to 'ayb_<appId>'.
     */
    constructor(options = {}) {
        if (!options.appId) throw new Error('appId is required');
        this.appId = options.appId;
        this.storageKey = options.storageKey || `ayb_${this.appId}`;
        this._config = null;
    }

    // ---- Config Management ----

    /**
     * Load saved config from localStorage.
     * @returns {boolean} True if config was found and loaded.
     */
    loadConfig() {
        const saved = localStorage.getItem(this.storageKey);
        if (saved) {
            this._config = JSON.parse(saved);
            return true;
        }
        return false;
    }

    /**
     * Parse a database URL and save config with the given token.
     * Accepts URLs in these formats:
     *   - https://host/entity/database
     *   - https://host/v1/entity/database
     *
     * @param {string} url - Database URL
     * @param {string} token - API token
     */
    saveConfig(url, token) {
        const parsed = AybClient.parseDatabaseUrl(url);
        this._config = { ...parsed, token };
        localStorage.setItem(this.storageKey, JSON.stringify(this._config));
    }

    /**
     * Clear stored config and disconnect.
     */
    clearConfig() {
        this._config = null;
        localStorage.removeItem(this.storageKey);
    }

    /**
     * @returns {boolean} True if config is loaded (connected).
     */
    isConnected() {
        return !!this._config;
    }

    /**
     * Get information about the current connection.
     * @returns {Object|null} Connection info or null if not connected.
     */
    getConnectionInfo() {
        if (!this._config) return null;
        return {
            baseUrl: this._config.baseUrl,
            entity: this._config.entity,
            database: this._config.database,
            databaseUrl: `${this._config.baseUrl}/v1/${this._config.entity}/${this._config.database}`
        };
    }

    // ---- Query ----

    /**
     * Execute a SQL query and return the raw response.
     * @param {string} sql - SQL query string
     * @returns {Promise<{fields: string[], rows: (string|null)[][]}>}
     */
    async query(sql) {
        if (!this._config) {
            throw new Error('Not connected. Call saveConfig() or loadConfig() first.');
        }

        const { baseUrl, entity, database, token } = this._config;
        const url = `${baseUrl}/v1/${entity}/${database}/query`;

        const response = await this._fetchWithRetry(url, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${token}`,
                'Content-Type': 'text/plain'
            },
            body: sql
        });

        if (!response.ok) {
            const text = await response.text();
            throw new Error(`Query failed: ${text}`);
        }

        return response.json();
    }

    /**
     * Execute a SQL query and return results as an array of objects.
     * Each object has keys matching the column names from the query.
     *
     * @param {string} sql - SQL query string
     * @returns {Promise<Object[]>} Array of row objects
     *
     * @example
     *   const todos = await db.queryObjects('SELECT id, title, done FROM todos');
     *   // [{id: '1', title: 'Buy milk', done: '0'}, ...]
     */
    async queryObjects(sql) {
        const result = await this.query(sql);
        if (!result.fields || !result.rows) return [];
        return result.rows.map(row => {
            const obj = {};
            result.fields.forEach((field, i) => {
                obj[field] = row[i];
            });
            return obj;
        });
    }

    // ---- Migrations ----

    /**
     * Run database migrations, scoped by this client's appId.
     * Multiple applications can share a single database without
     * migration conflicts.
     *
     * Migrations are run in order. Each migration is a SQL string.
     * Already-applied migrations are skipped. Idempotent errors
     * (duplicate column, table already exists) are ignored.
     *
     * @param {string[]} migrations - Array of SQL migration statements
     *
     * @example
     *   await db.runMigrations([
     *     `CREATE TABLE IF NOT EXISTS todos (
     *       id INTEGER PRIMARY KEY AUTOINCREMENT,
     *       title TEXT NOT NULL,
     *       done INTEGER DEFAULT 0
     *     )`,
     *     `ALTER TABLE todos ADD COLUMN position INTEGER DEFAULT 0`
     *   ]);
     */
    async runMigrations(migrations) {
        const appId = AybClient.escapeSQL(this.appId);

        // Create migrations table (shared across all apps)
        await this.query(`CREATE TABLE IF NOT EXISTS _ayb_migrations (
            app_id TEXT NOT NULL,
            version INTEGER NOT NULL,
            applied_at TEXT DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (app_id, version)
        )`);

        // Get current version for this app
        const result = await this.query(
            `SELECT MAX(version) FROM _ayb_migrations WHERE app_id = '${appId}'`
        );
        let currentVersion = parseInt(result.rows?.[0]?.[0], 10) || 0;

        // Auto-repair corrupted state
        if (currentVersion > migrations.length) {
            await this.query(`DELETE FROM _ayb_migrations WHERE app_id = '${appId}'`);
            currentVersion = 0;
        }

        // Run pending migrations
        for (let i = currentVersion; i < migrations.length; i++) {
            try {
                await this.query(migrations[i]);
            } catch (e) {
                const msg = e.message.toLowerCase();
                if (!msg.includes('duplicate column') && !msg.includes('already exists')) {
                    throw e;
                }
            }
            await this.query(
                `INSERT OR REPLACE INTO _ayb_migrations (app_id, version) VALUES ('${appId}', ${i + 1})`
            );
        }
    }

    /**
     * Load config from localStorage and optionally run migrations.
     * Throws if no saved config is found or if the connection fails.
     *
     * @param {string[]} [migrations] - Optional migrations to run after connecting
     * @returns {Promise<void>}
     */
    async connect(migrations) {
        if (!this.loadConfig()) {
            throw new Error('No saved configuration found. Call saveConfig() first.');
        }
        if (migrations && migrations.length > 0) {
            await this.runMigrations(migrations);
        }
    }

    // ---- Network ----

    /**
     * Fetch with automatic retry on network errors.
     * Retries up to maxRetries times with exponential backoff (2s, 4s, 8s, 16s).
     * Only retries on network errors (fetch throwing), not on HTTP error responses.
     *
     * @param {string} url
     * @param {Object} options - fetch options
     * @param {number} [maxRetries=4]
     * @returns {Promise<Response>}
     */
    async _fetchWithRetry(url, options, maxRetries = 4) {
        let lastError;
        for (let attempt = 0; attempt <= maxRetries; attempt++) {
            try {
                return await fetch(url, options);
            } catch (e) {
                lastError = e;
                if (attempt < maxRetries) {
                    const delay = Math.pow(2, attempt + 1) * 1000;
                    await new Promise(r => setTimeout(r, delay));
                }
            }
        }
        throw lastError;
    }

    // ---- Static Helpers ----

    /**
     * Escape a string for safe inclusion in SQL queries.
     * Replaces single quotes with doubled single quotes.
     *
     * @param {*} str - Value to escape (null/undefined become empty string)
     * @returns {string}
     *
     * @example
     *   const name = AybClient.escapeSQL("O'Brien");
     *   await db.query(`INSERT INTO users (name) VALUES ('${name}')`);
     */
    static escapeSQL(str) {
        if (str === null || str === undefined) return '';
        return String(str).replace(/'/g, "''");
    }

    /**
     * Parse a database URL into its components.
     *
     * @param {string} url - Database URL
     * @returns {{baseUrl: string, entity: string, database: string}}
     */
    static parseDatabaseUrl(url) {
        const urlObj = new URL(url);
        const pathParts = urlObj.pathname.split('/').filter(p => p);

        if (pathParts.length >= 3 && pathParts[0] === 'v1') {
            return { baseUrl: urlObj.origin, entity: pathParts[1], database: pathParts[2] };
        } else if (pathParts.length >= 2) {
            return { baseUrl: urlObj.origin, entity: pathParts[0], database: pathParts[1] };
        }
        throw new Error('Invalid database URL. Expected: https://host/entity/database');
    }
}


class AybOAuth extends AybClient {
    /**
     * @param {Object} options
     * @param {string} options.appName - Display name shown during authorization.
     *   Also used as the appId for config/migration scoping unless overridden.
     * @param {string} options.queryPermissionLevel - 'read-only' or 'read-write'
     * @param {string} [options.serverUrl] - Defaults to window.location.origin
     * @param {string} [options.appId] - Override appId (defaults to appName)
     * @param {string} [options.storageKey] - Override localStorage key prefix
     */
    constructor(options) {
        if (!options.appName) throw new Error('appName is required');
        if (!options.queryPermissionLevel) throw new Error('queryPermissionLevel is required');
        if (!['read-only', 'read-write'].includes(options.queryPermissionLevel)) {
            throw new Error('queryPermissionLevel must be "read-only" or "read-write"');
        }

        super({
            appId: options.appId || options.appName,
            storageKey: options.storageKey
        });

        this.serverUrl = options.serverUrl || window.location.origin;
        this.appName = options.appName;
        this.queryPermissionLevel = options.queryPermissionLevel;
    }

    /**
     * Check if we have valid stored credentials.
     * @returns {boolean}
     */
    isAuthenticated() {
        return this.isConnected();
    }

    /**
     * Get connection info including OAuth-specific metadata.
     * @returns {Object|null}
     */
    getConnectionInfo() {
        const base = super.getConnectionInfo();
        if (!base) return null;
        const meta = this._loadMeta();
        return { ...base, ...meta };
    }

    /**
     * Start the OAuth authorization flow. Redirects the browser.
     *
     * @param {Object} [options]
     * @param {string} [options.callbackPath] - Path for callback. Defaults to current path.
     */
    async authorize(options = {}) {
        const codeVerifier = this._generateCodeVerifier();
        const codeChallenge = await this._sha256(codeVerifier);
        const state = this._generateState();

        sessionStorage.setItem('ayb_pkce_verifier', codeVerifier);
        sessionStorage.setItem('ayb_oauth_state', state);
        sessionStorage.setItem('ayb_oauth_server', this.serverUrl);

        const callbackUrl = options.callbackPath
            ? window.location.origin + options.callbackPath
            : window.location.origin + window.location.pathname;

        const params = new URLSearchParams({
            response_type: 'code',
            redirect_uri: callbackUrl,
            scope: this.queryPermissionLevel,
            state: state,
            code_challenge: codeChallenge,
            code_challenge_method: 'S256',
            app_name: this.appName
        });

        window.location.href = `${this.serverUrl}/oauth/authorize?${params}`;
    }

    /**
     * Handle the OAuth callback. Call this on page load.
     * @returns {Promise<boolean>} True if callback was handled successfully.
     */
    async handleCallback() {
        const params = new URLSearchParams(window.location.search);
        const code = params.get('code');
        const state = params.get('state');
        const error = params.get('error');

        if (!code && !error) {
            return false;
        }

        if (error) {
            this._cleanUrl();
            throw new Error(`Authorization failed: ${error}`);
        }

        const savedState = sessionStorage.getItem('ayb_oauth_state');
        if (state !== savedState) {
            this._cleanUrl();
            throw new Error('State mismatch - possible CSRF attack');
        }

        const serverUrl = sessionStorage.getItem('ayb_oauth_server') || this.serverUrl;
        const codeVerifier = sessionStorage.getItem('ayb_pkce_verifier');

        if (!codeVerifier) {
            this._cleanUrl();
            throw new Error('Missing PKCE verifier - authorization flow may have been interrupted');
        }

        // Exchange code for token
        const response = await this._fetchWithRetry(`${serverUrl}/v1/oauth/token`, {
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
            const errorData = await response.json().catch(() => ({}));
            this._cleanUrl();
            throw new Error(errorData.error_description || 'Token exchange failed');
        }

        const tokenData = await response.json();

        // Store config in unified format
        const parsed = AybClient.parseDatabaseUrl(tokenData.database_url);
        this._config = { ...parsed, token: tokenData.access_token };
        localStorage.setItem(this.storageKey, JSON.stringify(this._config));

        // Store OAuth-specific metadata separately
        this._saveMeta({
            database: tokenData.database,
            queryPermissionLevel: tokenData.query_permission_level
        });

        // Clean up
        sessionStorage.removeItem('ayb_pkce_verifier');
        sessionStorage.removeItem('ayb_oauth_state');
        sessionStorage.removeItem('ayb_oauth_server');
        this._cleanUrl();

        return true;
    }

    /**
     * Disconnect and clear all stored credentials.
     */
    disconnect() {
        this.clearConfig();
        localStorage.removeItem(`${this.storageKey}_meta`);
    }

    // ---- Private helpers ----

    _saveMeta(meta) {
        localStorage.setItem(`${this.storageKey}_meta`, JSON.stringify(meta));
    }

    _loadMeta() {
        const saved = localStorage.getItem(`${this.storageKey}_meta`);
        return saved ? JSON.parse(saved) : {};
    }

    _generateCodeVerifier() {
        const array = new Uint8Array(32);
        crypto.getRandomValues(array);
        return this._base64UrlEncode(array);
    }

    _generateState() {
        const array = new Uint8Array(16);
        crypto.getRandomValues(array);
        return this._base64UrlEncode(array);
    }

    async _sha256(str) {
        const encoder = new TextEncoder();
        const data = encoder.encode(str);
        const hash = await crypto.subtle.digest('SHA-256', data);
        return this._base64UrlEncode(new Uint8Array(hash));
    }

    _base64UrlEncode(array) {
        return btoa(String.fromCharCode(...array))
            .replace(/\+/g, '-')
            .replace(/\//g, '_')
            .replace(/=+$/, '');
    }

    _cleanUrl() {
        const url = new URL(window.location.href);
        url.searchParams.delete('code');
        url.searchParams.delete('state');
        url.searchParams.delete('error');
        window.history.replaceState({}, '', url.pathname + url.search);
    }
}


// Export for different module systems
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { AybClient, AybOAuth };
}
if (typeof window !== 'undefined') {
    window.AybClient = AybClient;
    window.AybOAuth = AybOAuth;
}
