export type AybClientOptions = {
    /**
     * - Application identifier, used to scope
     * localStorage keys and migration state. Required.
     */
    appId: string;
    /**
     * - localStorage key prefix.
     * Defaults to 'ayb_<appId>'.
     */
    storageKey?: string;
};
export type ConnectionInfo = {
    /**
     * - Server origin URL
     */
    baseUrl: string;
    /**
     * - Entity (user/org) slug
     */
    entity: string;
    /**
     * - Database slug
     */
    database: string;
    /**
     * - Full database API URL
     */
    databaseUrl: string;
};
export type QueryResult = {
    /**
     * - Column names
     */
    fields: string[];
    /**
     * - Row data
     */
    rows: (string | null)[][];
};
export type AybOAuthOptions = {
    /**
     * - Display name shown during authorization.
     * Also used as the appId for config/migration scoping unless overridden.
     */
    appName: string;
    /**
     * - Permission level to request
     */
    queryPermissionLevel: "read-only" | "read-write";
    /**
     * - The ayb server URL (e.g. 'https://thedata.zone')
     */
    serverUrl: string;
    /**
     * - Override appId (defaults to appName)
     */
    appId?: string;
    /**
     * - Override localStorage key prefix
     */
    storageKey?: string;
};
export type ServerSelectionModalOptions = {
    /**
     * - Display name shown during authorization
     */
    appName: string;
    /**
     * - Permission level to request
     */
    queryPermissionLevel: "read-only" | "read-write";
    /**
     * - Server URLs for the dropdown.
     * Defaults to ['https://thedata.zone'].
     */
    serverUrls?: string[];
    /**
     * - Override appId (defaults to appName)
     */
    appId?: string;
    /**
     * - Override localStorage key prefix
     */
    storageKey?: string;
};
/**
 * ayb.js - Client library for building apps on ayb (https://github.com/marcua/ayb)
 *
 * NOTE: When changing public API, regenerate ayb.d.ts:
 *   cd client-js && npm run generate-types
 *
 * Include via <script src="ayb.js"></script>. Provides AybClient and AybOAuth.
 *
 * --- OAuth flow (recommended) ---
 *
 * On page load, check for a returning user or OAuth callback:
 *
 *   const OAUTH_OPTIONS = {
 *     appName: 'My App',
 *     queryPermissionLevel: 'read-write',   // 'read-only' or 'read-write'
 *   };
 *   const STORAGE_KEY = 'ayb_MyApp';
 *   const params = new URLSearchParams(window.location.search);
 *   const saved = localStorage.getItem(STORAGE_KEY);
 *   let ayb = null;
 *
 *   if (params.has('code') || params.has('error')) {
 *     // Returning from OAuth: the authorize() call stored the server URL
 *     // in sessionStorage before redirecting away (sessionStorage survives
 *     // same-tab navigations but is cleared when the tab closes, so PKCE
 *     // secrets don't linger).
 *     ayb = new AybOAuth({
 *       ...OAUTH_OPTIONS,
 *       serverUrl: sessionStorage.getItem('ayb_oauth_server'),
 *     });
 *     await ayb.handleCallback();
 *   } else if (saved) {
 *     // Returning user: restore saved connection
 *     ayb = new AybOAuth({
 *       ...OAUTH_OPTIONS,
 *       serverUrl: JSON.parse(saved).baseUrl,
 *     });
 *     ayb.loadConfig();
 *   }
 *
 * If authenticated, run migrations and query:
 *
 *   if (ayb && ayb.isAuthenticated()) {
 *     await ayb.runMigrations([
 *       'CREATE TABLE IF NOT EXISTS todos (id INTEGER PRIMARY KEY, title TEXT, done INTEGER DEFAULT 0)',
 *     ]);
 *     const todos = await ayb.queryObjects('SELECT * FROM todos');
 *   }
 *
 * For first-time users, show a server selection modal:
 *
 *   connectButton.onclick = () => {
 *     AybOAuth.createServerSelectionModal({
 *       appName: 'My App',
 *       queryPermissionLevel: 'read-write',
 *       serverUrls: ['https://thedata.zone'],  // optional, this is the default
 *     });
 *   };
 *
 * To disconnect:
 *
 *   ayb.disconnect();
 *
 * --- Manual token auth ---
 *
 *   const db = new AybClient({ appId: 'my-app' });
 *   db.saveConfig('https://host/v1/entity/database', 'ayb_xxx_yyy');
 *   await db.runMigrations([...]);
 *   const rows = await db.queryObjects('SELECT * FROM todos');
 *   // On next page load: db.loadConfig() restores the saved connection.
 */
/**
 * @typedef {Object} AybClientOptions
 * @property {string} appId - Application identifier, used to scope
 *   localStorage keys and migration state. Required.
 * @property {string} [storageKey] - localStorage key prefix.
 *   Defaults to 'ayb_<appId>'.
 */
/**
 * @typedef {Object} ConnectionInfo
 * @property {string} baseUrl - Server origin URL
 * @property {string} entity - Entity (user/org) slug
 * @property {string} database - Database slug
 * @property {string} databaseUrl - Full database API URL
 */
/**
 * @typedef {Object} QueryResult
 * @property {string[]} fields - Column names
 * @property {(string|null)[][]} rows - Row data
 */
export class AybClient {
    /**
     * Escape a string for safe inclusion in a single-quoted SQL literal.
     *
     * This is appropriate for SQLite string literals: it doubles every
     * single-quote so that the value cannot break out of '...'.  It does
     * NOT protect against injection in other SQL contexts (e.g. outside
     * quotes, inside LIKE patterns, or in identifiers).
     *
     * Always wrap the result in single quotes:
     *   `WHERE name = '${AybClient.escapeSQL(input)}'`
     *
     * Never interpolate the result without surrounding quotes -- that
     * would allow numeric or keyword injection:
     *   // UNSAFE: `WHERE id = ${AybClient.escapeSQL(input)}`
     *
     * @param {*} str - Value to escape (null/undefined become empty string)
     * @returns {string}
     *
     * @example
     *   const name = AybClient.escapeSQL("O'Brien");
     *   await db.query(`INSERT INTO users (name) VALUES ('${name}')`);
     */
    static escapeSQL(str: any): string;
    /**
     * Parse a database URL into its components.
     *
     * @param {string} url - Database URL
     * @returns {{baseUrl: string, entity: string, database: string}} Parsed URL components
     */
    static parseDatabaseUrl(url: string): {
        baseUrl: string;
        entity: string;
        database: string;
    };
    /**
     * @param {AybClientOptions} [options]
     */
    constructor(options?: AybClientOptions);
    appId: string;
    storageKey: string;
    _config: any;
    /**
     * Load saved config from localStorage.
     * @returns {boolean} True if config was found and loaded.
     */
    loadConfig(): boolean;
    /**
     * Parse a database URL and save config with the given token.
     * Accepts URLs in these formats:
     *   - https://host/entity/database
     *   - https://host/v1/entity/database
     *
     * @param {string} url - Database URL
     * @param {string} token - API token
     */
    saveConfig(url: string, token: string): void;
    /**
     * Clear stored config and disconnect.
     */
    clearConfig(): void;
    /**
     * @returns {boolean} True if config is loaded (connected).
     */
    isConnected(): boolean;
    /**
     * Get information about the current connection.
     * @returns {ConnectionInfo|null} Connection info or null if not connected.
     */
    getConnectionInfo(): ConnectionInfo | null;
    /**
     * Execute a SQL query and return the raw response.
     * @param {string} sql - SQL query string
     * @returns {Promise<QueryResult>}
     */
    query(sql: string): Promise<QueryResult>;
    /**
     * Execute a SQL query and return results as an array of objects.
     * Each object has keys matching the column names from the query.
     *
     * @param {string} sql - SQL query string
     * @returns {Promise<Record<string, string|null>[]>} Array of row objects
     *
     * @example
     *   const todos = await db.queryObjects('SELECT id, title, done FROM todos');
     *   // [{id: '1', title: 'Buy milk', done: '0'}, ...]
     */
    queryObjects(sql: string): Promise<Record<string, string | null>[]>;
    /**
     * Run database migrations, scoped by this client's appId.
     * Multiple applications can share a single database without
     * migration conflicts.
     *
     * Versioning: each migration's version is its 1-based index in the
     * array (first migration = version 1, second = version 2, etc.).
     * The _ayb_migrations table records which versions have been applied.
     * On each call we fetch MAX(version) to find out how far we've gotten,
     * then run migrations[maxVersion] through migrations[length-1].
     *
     * IMPORTANT: the migrations array must be append-only once deployed.
     * Never reorder, edit, or remove entries that have already run against
     * a live database -- only add new entries at the end.
     *
     * Already-applied migrations are skipped. Idempotent errors
     * (duplicate column, table already exists) are ignored.
     *
     * @param {string[]} migrations - Append-only array of SQL migration statements
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
    runMigrations(migrations: string[]): Promise<void>;
    /**
     * Load config from localStorage and optionally run migrations.
     * Throws if no saved config is found or if the connection fails.
     *
     * @param {string[]} [migrations] - Optional migrations to run after connecting
     */
    connect(migrations?: string[]): Promise<void>;
    /**
     * Fetch with automatic retry on network errors.
     * Retries up to maxRetries times with exponential backoff (2s, 4s, 8s, 16s).
     * Only retries on network errors (fetch throwing), not on HTTP error responses.
     *
     * @param {string} url
     * @param {RequestInit} options - fetch options
     * @param {number} [maxRetries=4]
     * @returns {Promise<Response>}
     */
    _fetchWithRetry(url: string, options: RequestInit, maxRetries?: number): Promise<Response>;
}
/**
 * @typedef {Object} AybOAuthOptions
 * @property {string} appName - Display name shown during authorization.
 *   Also used as the appId for config/migration scoping unless overridden.
 * @property {'read-only'|'read-write'} queryPermissionLevel - Permission level to request
 * @property {string} serverUrl - The ayb server URL (e.g. 'https://thedata.zone')
 * @property {string} [appId] - Override appId (defaults to appName)
 * @property {string} [storageKey] - Override localStorage key prefix
 */
/**
 * @typedef {Object} ServerSelectionModalOptions
 * @property {string} appName - Display name shown during authorization
 * @property {'read-only'|'read-write'} queryPermissionLevel - Permission level to request
 * @property {string[]} [serverUrls] - Server URLs for the dropdown.
 *   Defaults to ['https://thedata.zone'].
 * @property {string} [appId] - Override appId (defaults to appName)
 * @property {string} [storageKey] - Override localStorage key prefix
 */
export class AybOAuth extends AybClient {
    /**
     * Show a server selection modal and start the OAuth flow.
     * Creates a <dialog> with a dropdown of server URLs and an "Other..."
     * option for entering a custom URL. On Connect, constructs an AybOAuth
     * instance with the selected server and calls authorize().
     *
     * @param {ServerSelectionModalOptions} options
     */
    static createServerSelectionModal(options: ServerSelectionModalOptions): void;
    /**
     * @param {AybOAuthOptions} options
     */
    constructor(options: AybOAuthOptions);
    serverUrl: string;
    appName: string;
    queryPermissionLevel: "read-only" | "read-write";
    /**
     * Check if we have valid stored credentials.
     * @returns {boolean}
     */
    isAuthenticated(): boolean;
    /**
     * Get connection info including OAuth-specific metadata.
     * @returns {(ConnectionInfo & {database: string, queryPermissionLevel: string})|null}
     */
    getConnectionInfo(): (ConnectionInfo & {
        database: string;
        queryPermissionLevel: string;
    }) | null;
    /**
     * Start the OAuth authorization flow. Redirects the browser.
     *
     * @param {{callbackPath?: string}} [options] - Authorization options
     */
    authorize(options?: {
        callbackPath?: string;
    }): Promise<void>;
    /**
     * Handle the OAuth callback. Call this on page load.
     * @returns {Promise<boolean>} True if callback was handled successfully.
     */
    handleCallback(): Promise<boolean>;
    /**
     * Disconnect and clear all stored credentials.
     */
    disconnect(): void;
    /**
     * @param {{database: string, queryPermissionLevel: string}} meta
     */
    _saveMeta(meta: {
        database: string;
        queryPermissionLevel: string;
    }): void;
    /**
     * @returns {{database?: string, queryPermissionLevel?: string}}
     */
    _loadMeta(): {
        database?: string;
        queryPermissionLevel?: string;
    };
    /** @returns {string} */
    _generateCodeVerifier(): string;
    /** @returns {string} */
    _generateState(): string;
    /**
     * @param {string} str
     * @returns {Promise<string>}
     */
    _sha256(str: string): Promise<string>;
    /**
     * @param {Uint8Array} array
     * @returns {string}
     */
    _base64UrlEncode(array: Uint8Array): string;
    _cleanUrl(): void;
}
