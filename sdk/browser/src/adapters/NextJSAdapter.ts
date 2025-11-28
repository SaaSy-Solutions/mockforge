/**
 * Next.js Adapter
 *
 * Integration adapter for Next.js applications
 */

import { ForgeConnect } from '../core/ForgeConnect';
import { ForgeConnectConfig } from '../types';

/**
 * Next.js adapter configuration
 */
export interface NextJSAdapterConfig extends ForgeConnectConfig {
    /**
     * Only enable in development mode (default: true)
     */
    devOnly?: boolean;

    /**
     * Environment variable name for MockForge URL (default: 'NEXT_PUBLIC_MOCKFORGE_URL')
     */
    envVarName?: string;
}

/**
 * Next.js adapter for ForgeConnect
 *
 * Provides integration with Next.js by:
 * - Only running in development mode
 * - Reading configuration from environment variables
 * - Intercepting Next.js API routes and fetch calls
 */
export class NextJSAdapter {
    private forgeConnect: ForgeConnect;
    private config: NextJSAdapterConfig;
    private initialized: boolean = false;

    constructor(config: NextJSAdapterConfig = {}) {
        this.config = {
            devOnly: config.devOnly !== false,
            envVarName: config.envVarName || 'NEXT_PUBLIC_MOCKFORGE_URL',
            ...config,
        };

        // Get server URL from environment if not provided
        if (!this.config.serverUrl && typeof window !== 'undefined') {
            const envUrl = (window as any).process?.env?.[this.config.envVarName] ||
                          (window as any).__NEXT_DATA__?.env?.[this.config.envVarName];
            if (envUrl) {
                this.config.serverUrl = envUrl;
            }
        }

        this.forgeConnect = new ForgeConnect(this.config);
    }

    /**
     * Initialize the adapter (should be called in _app.tsx or layout)
     */
    async initialize(): Promise<boolean> {
        // Check if we should run
        if (this.config.devOnly) {
            const isDev = typeof window !== 'undefined' &&
                         (window.location.hostname === 'localhost' ||
                          window.location.hostname === '127.0.0.1' ||
                          process.env.NODE_ENV === 'development');

            if (!isDev) {
                return false;
            }
        }

        if (this.initialized) {
            return this.forgeConnect.getConnectionStatus().connected;
        }

        this.initialized = true;
        return await this.forgeConnect.initialize();
    }

    /**
     * Get the underlying ForgeConnect instance
     */
    getForgeConnect(): ForgeConnect {
        return this.forgeConnect;
    }

    /**
     * Stop the adapter
     */
    stop(): void {
        this.forgeConnect.stop();
        this.initialized = false;
    }
}

/**
 * React hook for Next.js integration
 *
 * @example
 * ```tsx
 * // app/layout.tsx or pages/_app.tsx
 * import { useNextJSForgeConnect } from '@mockforge/forgeconnect/adapters/nextjs';
 *
 * export default function RootLayout({ children }) {
 *   useNextJSForgeConnect({
 *     mockMode: 'auto',
 *   });
 *
 *   return <html>{children}</html>;
 * }
 * ```
 */
export function useNextJSForgeConnect(config?: NextJSAdapterConfig) {
    const [adapter] = React.useState(() => new NextJSAdapter(config));
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
    console.warn('[NextJSAdapter] React is not available. useNextJSForgeConnect hook will not work.');
}
