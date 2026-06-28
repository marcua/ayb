export type ServerSelectionModalOptions = {
    appName: string;
    queryPermissionLevel: "read-only" | "read-write";
    serverUrls?: string[] | undefined;
    appId?: string | undefined;
    storageKey?: string | undefined;
};
export type AybOAuthOptions = {
    appName: string;
    queryPermissionLevel: "read-only" | "read-write";
    serverUrl: string;
    appId?: string | undefined;
    storageKey?: string | undefined;
};
export class AybClient {
    static escapeSQL(str: any): string;
    static parseDatabaseUrl(url: string): {
        baseUrl: string;
        entity: string;
        database: string;
    };
    constructor(options?: {
        appId: string;
        storageKey?: string | undefined;
        url?: string | undefined;
        token?: string | undefined;
    });
    appId: string;
    storageKey: string;
    _config: {
        token: string;
        baseUrl: string;
        entity: string;
        database: string;
    } | null;
    loadConfig(): boolean;
    saveConfig(url: string, token: string): void;
    disconnect(): void;
    isConnected(): boolean;
    getConnectionInfo(): {
        baseUrl: string;
        entity: string;
        database: string;
        databaseUrl: string;
    } | null;
    query(sql: string, maxRetries?: number): Promise<{
        fields: string[];
        rows: (string | null)[][];
    }>;
    queryObjects(sql: string): Promise<Record<string, string | null>[]>;
    _fetchWithRetry(url: string, options: RequestInit, maxRetries?: number): Promise<Response>;
}
export class AybOAuth extends AybClient {
    constructor(options: AybOAuthOptions);
    serverUrl: string;
    appName: string;
    queryPermissionLevel: "read-only" | "read-write";
    getConnectionInfo(): ({
        baseUrl: string;
        entity: string;
        database: string;
        databaseUrl: string;
    } & {
        queryPermissionLevel?: string;
    }) | null;
    authorize(options?: {
        callbackPath?: string;
    }): Promise<void>;
    handleCallback(): Promise<boolean>;
    _generateCodeVerifier(): string;
    _generateState(): string;
    _sha256(str: string): Promise<string>;
    _base64UrlEncode(array: Uint8Array): string;
    _cleanUrl(): void;
}
export function restoreOAuth(options: AybOAuthOptions): Promise<AybOAuth | null>;
export function createServerSelectionModal(options: ServerSelectionModalOptions): void;
export function runMigrations(client: AybClient, appId: string, migrations: string[]): Promise<void>;
