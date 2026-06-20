'use strict';

const test = require('node:test');
const assert = require('node:assert');

const { AybClient } = require('../ayb.js');

// A localStorage stand-in that records every access so tests can assert
// that the url+token constructor path never touches it.
function makeSpyLocalStorage() {
    const store = {};
    const touches = [];
    return {
        store,
        touches,
        getItem(key) {
            touches.push(['getItem', key]);
            return Object.prototype.hasOwnProperty.call(store, key) ? store[key] : null;
        },
        setItem(key, value) {
            touches.push(['setItem', key]);
            store[key] = String(value);
        },
        removeItem(key) {
            touches.push(['removeItem', key]);
            delete store[key];
        },
    };
}

test.beforeEach(() => {
    global.localStorage = makeSpyLocalStorage();
});

test.afterEach(() => {
    delete global.localStorage;
    delete global.fetch;
});

test('constructor with url + token connects without touching localStorage', async () => {
    const db = new AybClient({
        appId: 'my-app',
        url: 'https://host.example.com/v1/alice/notes',
        token: 'ayb_xxx_yyy',
    });

    assert.strictEqual(db.isConnected(), true);

    // Stub fetch and confirm query() uses the parsed connection details.
    let capturedUrl;
    let capturedOptions;
    global.fetch = async (url, options) => {
        capturedUrl = url;
        capturedOptions = options;
        return {
            ok: true,
            json: async () => ({ fields: ['n'], rows: [['1']] }),
        };
    };

    const result = await db.query('SELECT 1 AS n');
    assert.deepStrictEqual(result, { fields: ['n'], rows: [['1']] });
    assert.strictEqual(
        capturedUrl,
        'https://host.example.com/v1/alice/notes/query'
    );
    assert.strictEqual(capturedOptions.headers.Authorization, 'Bearer ayb_xxx_yyy');
    assert.strictEqual(capturedOptions.body, 'SELECT 1 AS n');

    // localStorage must never have been read or written.
    assert.deepStrictEqual(global.localStorage.touches, []);
});

test('constructor without url/token is not connected and supports saveConfig/loadConfig', () => {
    const db = new AybClient({ appId: 'my-app' });
    assert.strictEqual(db.isConnected(), false);

    // Existing saveConfig flow persists to localStorage.
    db.saveConfig('https://host.example.com/v1/alice/notes', 'ayb_xxx_yyy');
    assert.strictEqual(db.isConnected(), true);
    assert.ok(global.localStorage.store['ayb_my-app']);

    // A fresh client can restore the connection via loadConfig.
    const db2 = new AybClient({ appId: 'my-app' });
    assert.strictEqual(db2.isConnected(), false);
    assert.strictEqual(db2.loadConfig(), true);
    assert.strictEqual(db2.isConnected(), true);
    assert.deepStrictEqual(db2.getConnectionInfo(), db.getConnectionInfo());
});

test('constructor with only url or only token does not connect', () => {
    const urlOnly = new AybClient({ appId: 'my-app', url: 'https://host.example.com/v1/alice/notes' });
    assert.strictEqual(urlOnly.isConnected(), false);

    const tokenOnly = new AybClient({ appId: 'my-app', token: 'ayb_xxx_yyy' });
    assert.strictEqual(tokenOnly.isConnected(), false);
});

test('getConnectionInfo returns parsed components for url + token constructor', () => {
    const db = new AybClient({
        appId: 'my-app',
        url: 'https://host.example.com/v1/alice/notes',
        token: 'ayb_xxx_yyy',
    });

    assert.deepStrictEqual(db.getConnectionInfo(), {
        baseUrl: 'https://host.example.com',
        entity: 'alice',
        database: 'notes',
        databaseUrl: 'https://host.example.com/v1/alice/notes',
    });
});
