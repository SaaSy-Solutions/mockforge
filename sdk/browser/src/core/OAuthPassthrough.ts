/**
 * OAuth Passthrough
 *
 * Handles OAuth flows by bypassing mocking and preserving tokens
 */

export interface OAuthPassthroughConfig {
    /**
     * URLs or patterns to bypass mocking (e.g., OAuth endpoints)
     */
    bypassUrls?: string[];

    /**
     * Regex patterns for URLs to bypass
     */
    bypassPatterns?: RegExp[];

    /**
     * Function to determine if a request should bypass mocking
     */
    shouldBypass?: (url: string, method: string) => boolean;

    /**
     * Token storage key (default: 'forgeconnect_oauth_token')
     */
    tokenStorageKey?: string;

    /**
     * Auto-inject tokens into requests
     */
    autoInjectTokens?: boolean;

    /**
     * Token injection header name (default: 'Authorization')
     */
    tokenHeaderName?: string;
}

/**
 * Default OAuth endpoint patterns
 */
const DEFAULT_OAUTH_PATTERNS = [
    /\/oauth\/authorize/i,
    /\/oauth\/token/i,
    /\/oauth\/callback/i,
    /\/auth\/authorize/i,
    /\/auth\/token/i,
    /\/auth\/callback/i,
    /\/login/i,
    /\/logout/i,
    /\/token/i,
    /\/refresh/i,
];

/**
 * OAuth Passthrough Manager
 */
export class OAuthPassthrough {
    private config: Required<Pick<OAuthPassthroughConfig, 'tokenStorageKey' | 'autoInjectTokens' | 'tokenHeaderName'>> & OAuthPassthroughConfig;
    private storedToken: string | null = null;

    constructor(config?: OAuthPassthroughConfig) {
        this.config = {
            tokenStorageKey: config?.tokenStorageKey || 'forgeconnect_oauth_token',
            autoInjectTokens: config?.autoInjectTokens !== false,
            tokenHeaderName: config?.tokenHeaderName || 'Authorization',
            ...config,
        };

        // Load stored token
        this.loadStoredToken();
    }

    /**
     * Check if a request should bypass mocking
     */
    shouldBypass(url: string, method: string): boolean {
        // Use custom function if provided
        if (this.config.shouldBypass) {
            return this.config.shouldBypass(url, method);
        }

        // Check bypass URLs
        if (this.config.bypassUrls) {
            for (const bypassUrl of this.config.bypassUrls) {
                if (url.includes(bypassUrl)) {
                    return true;
                }
            }
        }

        // Check bypass patterns
        if (this.config.bypassPatterns) {
            for (const pattern of this.config.bypassPatterns) {
                if (pattern.test(url)) {
                    return true;
                }
            }
        }

        // Check default OAuth patterns
        for (const pattern of DEFAULT_OAUTH_PATTERNS) {
            if (pattern.test(url)) {
                return true;
            }
        }

        return false;
    }

    /**
     * Extract token from response
     */
    extractTokenFromResponse(response: Response | any, responseBody?: any): string | null {
        try {
            // Try to get token from response body first (most common case)
            let body = responseBody;

            // If body is a string, try to parse it
            if (typeof body === 'string') {
                try {
                    body = JSON.parse(body);
                } catch {
                    // Not JSON, might be form-encoded or plain text
                    // Try to extract token from URL-encoded format
                    if (body.includes('access_token=')) {
                        const match = body.match(/access_token=([^&]+)/);
                        if (match) {
                            return this.formatToken(decodeURIComponent(match[1]), 'Bearer');
                        }
                    }
                    return null;
                }
            }

            if (body) {
                // Check common token fields
                if (typeof body === 'string') {
                    try {
                        body = JSON.parse(body);
                    } catch {
                        // Not JSON
                    }
                }

                if (body && typeof body === 'object') {
                    // Common OAuth token response fields
                    const tokenFields = [
                        'access_token',
                        'accessToken',
                        'token',
                        'id_token',
                        'idToken',
                        'authToken',
                    ];

                    for (const field of tokenFields) {
                        if (body[field]) {
                            return this.formatToken(body[field], body.token_type || body.tokenType || 'Bearer');
                        }
                    }
                }
            }

            // Try to get token from headers
            if (response instanceof Response) {
                const authHeader = response.headers.get('Authorization');
                if (authHeader) {
                    return authHeader;
                }

                // Check for custom token headers
                const tokenHeaders = ['X-Auth-Token', 'X-Access-Token', 'X-Token'];
                for (const headerName of tokenHeaders) {
                    const token = response.headers.get(headerName);
                    if (token) {
                        return this.formatToken(token, 'Bearer');
                    }
                }
            }
        } catch (error) {
            console.warn('[OAuthPassthrough] Failed to extract token:', error);
        }

        return null;
    }

    /**
     * Format token with type prefix
     */
    private formatToken(token: string, type: string = 'Bearer'): string {
        if (token.startsWith('Bearer ') || token.startsWith('Basic ')) {
            return token;
        }
        return `${type} ${token}`;
    }

    /**
     * Store token
     */
    async storeToken(token: string): Promise<void> {
        this.storedToken = token;

        try {
            // Store in localStorage
            localStorage.setItem(this.config.tokenStorageKey!, token);
        } catch (error) {
            console.warn('[OAuthPassthrough] Failed to store token:', error);
        }
    }

    /**
     * Load stored token
     */
    private loadStoredToken(): void {
        try {
            const token = localStorage.getItem(this.config.tokenStorageKey!);
            if (token) {
                this.storedToken = token;
            }
        } catch (error) {
            console.warn('[OAuthPassthrough] Failed to load token:', error);
        }
    }

    /**
     * Get stored token
     */
    getStoredToken(): string | null {
        return this.storedToken;
    }

    /**
     * Clear stored token
     */
    async clearToken(): Promise<void> {
        this.storedToken = null;

        try {
            localStorage.removeItem(this.config.tokenStorageKey!);
        } catch (error) {
            console.warn('[OAuthPassthrough] Failed to clear token:', error);
        }
    }

    /**
     * Inject token into request headers
     */
    injectToken(headers: Headers | Record<string, string> | undefined): Headers | Record<string, string> {
        if (!this.config.autoInjectTokens || !this.storedToken) {
            return headers || {};
        }

        if (headers instanceof Headers) {
            // Don't override existing Authorization header
            if (!headers.has(this.config.tokenHeaderName!)) {
                headers.set(this.config.tokenHeaderName!, this.storedToken);
            }
            return headers;
        } else {
            const headersObj = headers || {};
            // Don't override existing Authorization header
            if (!headersObj[this.config.tokenHeaderName!]) {
                headersObj[this.config.tokenHeaderName!] = this.storedToken;
            }
            return headersObj;
        }
    }

    /**
     * Process response and extract/store token if found
     */
    async processResponse(response: Response, responseBody?: any): Promise<void> {
        // Try to extract token from response
        const token = this.extractTokenFromResponse(response, responseBody);

        if (token) {
            await this.storeToken(token);
        }
    }
}
