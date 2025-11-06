/**
 * React Query Adapter
 * 
 * Integration adapter for @tanstack/react-query
 */

import { ForgeConnect } from '../core/ForgeConnect';
import { ForgeConnectConfig } from '../types';

/**
 * React Query adapter configuration
 */
export interface ReactQueryAdapterConfig extends ForgeConnectConfig {
    /**
     * Auto-mock failed queries (default: true)
     */
    autoMockFailedQueries?: boolean;
    
    /**
     * Auto-mock queries that return errors (default: true)
     */
    autoMockQueryErrors?: boolean;
}

/**
 * React Query adapter for ForgeConnect
 * 
 * Provides seamless integration with React Query by automatically
 * creating mocks for failed queries.
 */
export class ReactQueryAdapter {
    private forgeConnect: ForgeConnect;
    private config: ReactQueryAdapterConfig;

    constructor(config: ReactQueryAdapterConfig = {}) {
        this.config = {
            autoMockFailedQueries: config.autoMockFailedQueries !== false,
            autoMockQueryErrors: config.autoMockQueryErrors !== false,
            ...config,
        };

        this.forgeConnect = new ForgeConnect(this.config);
    }

    /**
     * Initialize the adapter
     */
    async initialize(): Promise<boolean> {
        return await this.forgeConnect.initialize();
    }

    /**
     * Get the underlying ForgeConnect instance
     */
    getForgeConnect(): ForgeConnect {
        return this.forgeConnect;
    }

    /**
     * Create a React Query error handler that auto-creates mocks
     */
    createErrorHandler() {
        return (error: any, query: any) => {
            if (this.config.autoMockQueryErrors && error) {
                // Extract request information from query
                const queryKey = query.queryKey;
                const queryFn = query.queryFn;

                // Try to extract URL from error or query
                let url: string | undefined;
                if (error?.config?.url) {
                    url = error.config.url;
                } else if (error?.request?.responseURL) {
                    url = error.request.responseURL;
                } else if (typeof queryFn === 'function') {
                    // Try to infer from query function
                    try {
                        const fnString = queryFn.toString();
                        const urlMatch = fnString.match(/['"`]([^'"`]+)['"`]/);
                        if (urlMatch) {
                            url = urlMatch[1];
                        }
                    } catch {
                        // Ignore
                    }
                }

                if (url) {
                    // Create a captured request from the error
                    const method = error?.config?.method?.toUpperCase() || 'GET';
                    const path = new URL(url, window.location.origin).pathname;

                    this.forgeConnect.createMockFromRequest({
                        method,
                        url,
                        path,
                        error: {
                            type: 'network',
                            message: error?.message || 'Query failed',
                        },
                        timestamp: Date.now(),
                    }).catch(err => {
                        console.warn('[ReactQueryAdapter] Failed to create mock:', err);
                    });
                }
            }
        };
    }

    /**
     * Stop the adapter
     */
    stop(): void {
        this.forgeConnect.stop();
    }
}

/**
 * Hook for using ForgeConnect with React Query
 * 
 * @example
 * ```tsx
 * function App() {
 *   const forgeConnect = useForgeConnect({
 *     mockMode: 'auto',
 *   });
 *   
 *   return <div>Your app</div>;
 * }
 * ```
 */
export function useForgeConnect(config?: ReactQueryAdapterConfig) {
    const [adapter] = React.useState(() => new ReactQueryAdapter(config));
    const [connected, setConnected] = React.useState(false);

    React.useEffect(() => {
        adapter.initialize().then(setConnected);
        return () => adapter.stop();
    }, []);

    return {
        adapter,
        forgeConnect: adapter.getForgeConnect(),
        connected,
    };
}

// Type guard for React availability
declare const React: any;
if (typeof React === 'undefined') {
    console.warn('[ReactQueryAdapter] React is not available. useForgeConnect hook will not work.');
}

