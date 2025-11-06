/**
 * Vue.js Adapter
 * 
 * Integration adapter for Vue.js applications
 */

import { ForgeConnect } from '../core/ForgeConnect';
import { ForgeConnectConfig } from '../types';

/**
 * Vue.js adapter configuration
 */
export interface VueAdapterConfig extends ForgeConnectConfig {
    /**
     * Only enable in development mode (default: true)
     */
    devOnly?: boolean;
    
    /**
     * Vue instance (optional, will try to auto-detect)
     */
    vue?: any;
}

/**
 * Vue.js adapter for ForgeConnect
 * 
 * Provides integration with Vue.js by:
 * - Only running in development mode
 * - Providing Vue composable for easy setup
 * - Auto-initialization on app mount
 */
export class VueAdapter {
    private forgeConnect: ForgeConnect;
    private config: VueAdapterConfig;
    private initialized: boolean = false;

    constructor(config: VueAdapterConfig = {}) {
        this.config = {
            devOnly: config.devOnly !== false,
            ...config,
        };

        this.forgeConnect = new ForgeConnect(this.config);
    }

    /**
     * Initialize the adapter
     */
    async initialize(): Promise<boolean> {
        // Check if we should run
        if (this.config.devOnly) {
            const isDev = typeof window !== 'undefined' && 
                         (window.location.hostname === 'localhost' || 
                          window.location.hostname === '127.0.0.1' ||
                          import.meta.env?.MODE === 'development' ||
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
    async stop(): Promise<void> {
        await this.forgeConnect.stop();
        this.initialized = false;
    }
}

/**
 * Vue composable for ForgeConnect
 * 
 * @example
 * ```vue
 * <script setup>
 * import { useForgeConnect } from '@mockforge/forgeconnect/adapters/vue';
 * 
 * const { forgeConnect, connected } = useForgeConnect({
 *   mockMode: 'auto',
 * });
 * </script>
 * ```
 */
export function useForgeConnect(config?: VueAdapterConfig) {
    // Try to import Vue
    let Vue: any;
    try {
        Vue = require('vue');
    } catch {
        // Vue not available, use basic implementation
        const adapter = new VueAdapter(config);
        const connected = Vue?.ref ? Vue.ref(false) : { value: false };
        
        adapter.initialize().then((isConnected) => {
            if (connected.value !== undefined) {
                connected.value = isConnected;
            }
        });

        return {
            adapter,
            forgeConnect: adapter.getForgeConnect(),
            connected: connected.value !== undefined ? connected : { value: false },
        };
    }

    // Vue 3 Composition API
    if (Vue.ref) {
        const adapter = new VueAdapter(config);
        const connected = Vue.ref(false);
        const error = Vue.ref<string | null>(null);

        Vue.onMounted(async () => {
            try {
                const result = await adapter.initialize();
                connected.value = result;
            } catch (err) {
                error.value = err instanceof Error ? err.message : 'Unknown error';
            }
        });

        Vue.onUnmounted(() => {
            adapter.stop();
        });

        return {
            adapter,
            forgeConnect: adapter.getForgeConnect(),
            connected,
            error,
        };
    }

    // Vue 2 Options API fallback
    const adapter = new VueAdapter(config);
    return {
        adapter,
        forgeConnect: adapter.getForgeConnect(),
        connected: { value: false },
    };
}

