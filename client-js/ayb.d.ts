export type AybOAuthOptions = {
    appName: string;
    queryPermissionLevel: "read-only" | "read-write";
    serverUrl: string;
    appId?: string;
    storageKey?: string;
};
export type ServerSelectionModalOptions = {
    appName: string;
    queryPermissionLevel: "read-only" | "read-write";
    serverUrls?: string[];
    appId?: string;
    storageKey?: string;
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
        storageKey?: string;
    });
    appId: string;
    storageKey: string;
    _config: any;
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
    runMigrations(migrations: string[]): Promise<void>;
    connect(migrations?: string[]): Promise<void>;
    _fetchWithRetry(url: string, options: RequestInit, maxRetries?: number): Promise<Response>;
}
export class AybOAuth extends AybClient {
    static createServerSelectionModal(options: ServerSelectionModalOptions): void;
    constructor(options: AybOAuthOptions);
    serverUrl: string;
    appName: string;
    queryPermissionLevel: "read-only" | "read-write";
    isAuthenticated(): boolean;
    getConnectionInfo(): ({
        baseUrl: string;
        entity: string;
        database: string;
        databaseUrl: string;
    } & {
        database: string;
        queryPermissionLevel: string;
    }) | null;
    authorize(options?: {
        callbackPath?: string;
    }): Promise<void>;
    handleCallback(): Promise<boolean>;
    _saveMeta(meta: {
        database: string;
        queryPermissionLevel: string;
    }): void;
    _loadMeta(): {
        database?: string;
        queryPermissionLevel?: string;
    };
    _generateState(): string;
    _sha256(str: string): Promise<string>;
    _base64UrlEncode(array: Uint8Array): string;
    _cleanUrl(): void;
}
