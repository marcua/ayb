/**
 * ayb-oauth.js - Client library for OAuth integration with ayb
 *
 * This library helps frontend applications integrate with ayb's OAuth-like
 * authorization flow to get scoped database access tokens.
 *
 * Usage:
 *   const ayb = new AybOAuth({
 *     appName: 'My App',
 *     queryPermissionLevel: 'read-write',
 *     serverUrl: 'https://your-ayb-server.com'
 *   });
 *
 *   // On page load, handle OAuth callback
 *   if (await ayb.handleCallback()) {
 *     console.log('Authenticated!');
 *   }
 *
 *   // Start OAuth flow
 *   if (!ayb.isAuthenticated()) {
 *     ayb.authorize();
 *   }
 *
 *   // Make queries
 *   const result = await ayb.query('SELECT * FROM users');
 */

class AybOAuth {
    /**
     * @param {Object} options
     * @param {string} options.appName - Required. Display name shown to users during authorization
     * @param {string} options.queryPermissionLevel - Required. 'read-only' or 'read-write'
     * @param {string} [options.serverUrl] - Optional. Defaults to window.location.origin
     * @param {string} [options.storageKey] - Optional. localStorage key. Defaults to 'ayb_auth'
     */
    constructor(options) {
        if (!options.appName) throw new Error('appName is required');
        if (!options.queryPermissionLevel) throw new Error('queryPermissionLevel is required');
        if (!['read-only', 'read-write'].includes(options.queryPermissionLevel)) {
            throw new Error('queryPermissionLevel must be "read-only" or "read-write"');
        }

        this.serverUrl = options.serverUrl || window.location.origin;
        this.appName = options.appName;
        this.queryPermissionLevel = options.queryPermissionLevel;
        this.storageKey = options.storageKey || 'ayb_auth';
    }

    /**
     * Check if we have valid stored credentials
     * @returns {boolean}
     */
    isAuthenticated() {
        return !!this.getToken();
    }

    /**
     * Get the stored authentication data
     * @returns {Object|null}
     */
    getToken() {
        const auth = localStorage.getItem(this.storageKey);
        return auth ? JSON.parse(auth) : null;
    }

    /**
     * Start the OAuth authorization flow
     * @param {Object} [options]
     * @param {string} [options.callbackPath] - Path for OAuth callback. Defaults to current path
     */
    async authorize(options = {}) {
        const codeVerifier = this.generateCodeVerifier();
        const codeChallenge = await this.sha256(codeVerifier);
        const state = this.generateState();

        // Store verifier and state for later verification
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
     * @returns {Promise<boolean>} True if this was a successful callback, false otherwise
     */
    async handleCallback() {
        const params = new URLSearchParams(window.location.search);
        const code = params.get('code');
        const state = params.get('state');
        const error = params.get('error');

        // Check if this is an OAuth callback
        if (!code && !error) {
            return false;
        }

        if (error) {
            // Clean up URL
            this.cleanUrl();
            throw new Error(`Authorization failed: ${error}`);
        }

        // Verify state to prevent CSRF attacks
        const savedState = sessionStorage.getItem('ayb_oauth_state');
        if (state !== savedState) {
            this.cleanUrl();
            throw new Error('State mismatch - possible CSRF attack');
        }

        // Get the server URL and code verifier
        const serverUrl = sessionStorage.getItem('ayb_oauth_server') || this.serverUrl;
        const codeVerifier = sessionStorage.getItem('ayb_pkce_verifier');

        if (!codeVerifier) {
            this.cleanUrl();
            throw new Error('Missing PKCE verifier - authorization flow may have been interrupted');
        }

        // Exchange code for token
        const response = await fetch(`${serverUrl}/v1/oauth/token`, {
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
            this.cleanUrl();
            throw new Error(errorData.error_description || 'Token exchange failed');
        }

        const tokenData = await response.json();

        // Store the token
        localStorage.setItem(this.storageKey, JSON.stringify({
            serverUrl: serverUrl,
            token: tokenData.access_token,
            database: tokenData.database,
            databaseUrl: tokenData.database_url,
            queryPermissionLevel: tokenData.query_permission_level
        }));

        // Clean up
        sessionStorage.removeItem('ayb_pkce_verifier');
        sessionStorage.removeItem('ayb_oauth_state');
        sessionStorage.removeItem('ayb_oauth_server');
        this.cleanUrl();

        return true;
    }

    /**
     * Execute a SQL query against the authorized database
     * @param {string} sql - The SQL query to execute
     * @returns {Promise<Object>} Query results
     */
    async query(sql) {
        const auth = this.getToken();
        if (!auth) {
            throw new Error('Not authenticated. Call authorize() first.');
        }

        const response = await fetch(`${auth.databaseUrl}/query`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${auth.token}`,
                'Content-Type': 'text/plain'
            },
            body: sql
        });

        if (!response.ok) {
            const errorData = await response.json().catch(() => ({}));
            throw new Error(errorData.message || 'Query failed');
        }

        return response.json();
    }

    /**
     * Disconnect and clear stored credentials
     */
    disconnect() {
        localStorage.removeItem(this.storageKey);
    }

    /**
     * Get information about the current connection
     * @returns {Object|null}
     */
    getConnectionInfo() {
        const auth = this.getToken();
        if (!auth) return null;
        return {
            database: auth.database,
            databaseUrl: auth.databaseUrl,
            serverUrl: auth.serverUrl,
            queryPermissionLevel: auth.queryPermissionLevel
        };
    }

    // ---- Helper methods ----

    /**
     * Generate a random code verifier for PKCE
     * @returns {string}
     */
    generateCodeVerifier() {
        const array = new Uint8Array(32);
        crypto.getRandomValues(array);
        return this.base64UrlEncode(array);
    }

    /**
     * Generate a random state parameter
     * @returns {string}
     */
    generateState() {
        const array = new Uint8Array(16);
        crypto.getRandomValues(array);
        return this.base64UrlEncode(array);
    }

    /**
     * Compute SHA-256 hash and return as base64url
     * @param {string} str
     * @returns {Promise<string>}
     */
    async sha256(str) {
        const encoder = new TextEncoder();
        const data = encoder.encode(str);
        const hash = await crypto.subtle.digest('SHA-256', data);
        return this.base64UrlEncode(new Uint8Array(hash));
    }

    /**
     * Base64 URL encode without padding
     * @param {Uint8Array} array
     * @returns {string}
     */
    base64UrlEncode(array) {
        return btoa(String.fromCharCode(...array))
            .replace(/\+/g, '-')
            .replace(/\//g, '_')
            .replace(/=+$/, '');
    }

    /**
     * Remove OAuth parameters from URL
     */
    cleanUrl() {
        const url = new URL(window.location.href);
        url.searchParams.delete('code');
        url.searchParams.delete('state');
        url.searchParams.delete('error');
        window.history.replaceState({}, '', url.pathname + url.search);
    }
}

// Export for different module systems
if (typeof module !== 'undefined' && module.exports) {
    module.exports = AybOAuth;
}
if (typeof window !== 'undefined') {
    window.AybOAuth = AybOAuth;
}
