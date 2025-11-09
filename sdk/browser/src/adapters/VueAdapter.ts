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
 * Full-featured implementation with all SDK features
 *
 * @example
 * ```vue
 * <script setup>
 * import { useForgeConnect } from '@mockforge/forgeconnect/adapters/vue';
 * import { ref, onMounted, onUnmounted, readonly } from 'vue';
 *
 * const {
 *   forgeConnect,
 *   connected,
 *   mocks,
 *   environments,
 *   activeEnvironment,
 *   offline,
 *   loading,
 *   error,
 *   refreshMocks,
 *   setActiveEnvironment
 * } = useForgeConnect({
 *   mockMode: 'auto',
 * });
 * </script>
 * ```
 */
export function useForgeConnect(config?: VueAdapterConfig) {
    // Try to import Vue using dynamic imports
    // This will be resolved at runtime
    let Vue: any = null;
    let vueModule: any = null;

    // Check if Vue is available in global scope (common in browser environments)
    if (typeof window !== 'undefined') {
        Vue = (window as any).Vue || (window as any).vue;
    }

    // If not in global scope, try dynamic import (will be async)
    // For now, we'll use a synchronous approach that works in most environments
    // The actual Vue instance will be detected when the composable runs
    const getVue = () => {
        if (Vue) return Vue;

        // Try to access Vue from module system (works in bundlers)
        try {
            // @ts-ignore - dynamic module access
            if (typeof require !== 'undefined') {
                try {
                    return require('vue');
                } catch {
                    try {
                        return require('@vue/composition-api');
                    } catch {
                        return null;
                    }
                }
            }
        } catch {
            // Ignore
        }

        return null;
    };

    Vue = getVue();

    if (!Vue) {
        // Vue not available, use basic implementation
        const adapter = new VueAdapter(config);
        const connected = { value: false };

        adapter.initialize().then((isConnected) => {
            connected.value = isConnected;
        });

        return {
            adapter,
            forgeConnect: adapter.getForgeConnect(),
            connected,
            mocks: { value: [] },
            environments: { value: [] },
            activeEnvironment: { value: null },
            offline: { value: false },
            loading: { value: false },
            error: { value: null },
            refreshMocks: async () => {},
            refreshEnvironments: async () => {},
            createMock: async () => null,
            setActiveEnvironment: async () => {},
        };
    }

    // Vue 3 Composition API (full implementation)
    if (Vue.ref && Vue.onMounted && Vue.onUnmounted) {
        const adapter = new VueAdapter(config);
        const forgeConnect = adapter.getForgeConnect();

        const connected = Vue.ref(false);
        const mocks = Vue.ref<any[]>([]);
        const environments = Vue.ref<any[]>([]);
        const activeEnvironment = Vue.ref<any | null>(null);
        const offline = Vue.ref(false);
        const loading = Vue.ref(false);
        const error = Vue.ref<string | null>(null);
        const liveReloadEnabled = Vue.ref(true);

        const initialize = async () => {
            loading.value = true;
            try {
                const result = await adapter.initialize();
                connected.value = result;

                if (result) {
                    await refreshMocks();
                    await refreshEnvironments();

                    // Watch connection status for offline detection
                    const status = forgeConnect.getConnectionStatus();
                    offline.value = !status.connected;

                    // Watch for live reload events
                    if (liveReloadEnabled.value) {
                        // Set up WebSocket listeners if enabled
                        if (config?.enableWebSocket) {
                            // WebSocket events are handled by ForgeConnect internally
                        }
                    }
                }
            } catch (err) {
                error.value = err instanceof Error ? err.message : 'Unknown error';
            } finally {
                loading.value = false;
            }
        };

        const refreshMocks = async () => {
            try {
                const mockList = await forgeConnect.listMocks();
                mocks.value = mockList;
            } catch (err) {
                console.error('[VueAdapter] Failed to refresh mocks:', err);
            }
        };

        const refreshEnvironments = async () => {
            try {
                const envList = await forgeConnect.listEnvironments();
                environments.value = envList;
                const active = await forgeConnect.getActiveEnvironment();
                activeEnvironment.value = active;
            } catch (err) {
                console.error('[VueAdapter] Failed to refresh environments:', err);
            }
        };

        const createMock = async (request: any) => {
            try {
                const mock = await forgeConnect.createMockFromRequest(request);
                await refreshMocks();
                return mock;
            } catch (err) {
                console.error('[VueAdapter] Failed to create mock:', err);
                throw err;
            }
        };

        const setActiveEnvironment = async (envId: string) => {
            try {
                await forgeConnect.setActiveEnvironment(envId);
                await refreshEnvironments();
            } catch (err) {
                console.error('[VueAdapter] Failed to set active environment:', err);
                throw err;
            }
        };

        Vue.onMounted(initialize);
        Vue.onUnmounted(() => {
            adapter.stop();
        });

        // Watch connection status
        const checkConnection = () => {
            const status = forgeConnect.getConnectionStatus();
            connected.value = status.connected;
            offline.value = !status.connected;
        };

        const connectionInterval = setInterval(checkConnection, 5000);
        Vue.onUnmounted(() => {
            clearInterval(connectionInterval);
        });

        return {
            adapter,
            forgeConnect: Vue.readonly(Vue.ref(forgeConnect)),
            connected: Vue.readonly(connected),
            mocks: Vue.readonly(mocks),
            environments: Vue.readonly(environments),
            activeEnvironment: Vue.readonly(activeEnvironment),
            offline: Vue.readonly(offline),
            loading: Vue.readonly(loading),
            error: Vue.readonly(error),
            liveReloadEnabled,
            initialize,
            refreshMocks,
            refreshEnvironments,
            createMock,
            setActiveEnvironment,
        };
    }

    // Vue 2 Options API fallback (basic)
    const adapter = new VueAdapter(config);
    return {
        adapter,
        forgeConnect: adapter.getForgeConnect(),
        connected: { value: false },
        mocks: { value: [] },
        environments: { value: [] },
        activeEnvironment: { value: null },
        offline: { value: false },
        loading: { value: false },
        error: { value: null },
        refreshMocks: async () => {},
        refreshEnvironments: async () => {},
        createMock: async () => null,
        setActiveEnvironment: async () => {},
    };
}
