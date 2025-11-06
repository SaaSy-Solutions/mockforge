/**
 * Vanilla JavaScript Adapter
 *
 * Simple adapter for vanilla JavaScript applications
 */

import { ForgeConnect } from '../core/ForgeConnect';
import { ForgeConnectConfig } from '../types';

/**
 * Vanilla adapter configuration
 */
export interface VanillaAdapterConfig extends ForgeConnectConfig {
    /**
     * Auto-initialize on load (default: false)
     */
    autoInit?: boolean;
}

/**
 * Vanilla JavaScript adapter for ForgeConnect
 *
 * Provides a simple way to use ForgeConnect in vanilla JavaScript
 * without any framework dependencies.
 */
export class VanillaAdapter {
    private forgeConnect: ForgeConnect;
    private config: VanillaAdapterConfig;

    constructor(config: VanillaAdapterConfig = {}) {
        this.config = {
            autoInit: config.autoInit || false,
            ...config,
        };

        this.forgeConnect = new ForgeConnect(this.config);

        // Auto-initialize if requested
        if (this.config.autoInit && typeof window !== 'undefined') {
            if (document.readyState === 'loading') {
                document.addEventListener('DOMContentLoaded', () => {
                    this.initialize().catch(err => {
                        console.error('[VanillaAdapter] Failed to initialize:', err);
                    });
                });
            } else {
                this.initialize().catch(err => {
                    console.error('[VanillaAdapter] Failed to initialize:', err);
                });
            }
        }
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
     * Stop the adapter
     */
    stop(): void {
        this.forgeConnect.stop();
    }
}

/**
 * Global initialization function for vanilla JavaScript
 *
 * @example
 * ```html
 * <script type="module">
 *   import { initForgeConnect } from '@mockforge/forgeconnect/adapters/vanilla';
 *
 *   const adapter = initForgeConnect({
 *     mockMode: 'auto',
 *   });
 *
 *   await adapter.initialize();
 * </script>
 * ```
 */
export function initForgeConnect(config?: VanillaAdapterConfig): VanillaAdapter {
    return new VanillaAdapter(config);
}

/**
 * Auto-initialize ForgeConnect when script loads
 *
 * @example
 * ```html
 * <script type="module">
 *   import { autoInitForgeConnect } from '@mockforge/forgeconnect/adapters/vanilla';
 *
 *   autoInitForgeConnect({
 *     mockMode: 'auto',
 *   });
 * </script>
 * ```
 */
export function autoInitForgeConnect(config?: VanillaAdapterConfig): VanillaAdapter {
    return new VanillaAdapter({
        ...config,
        autoInit: true,
    });
}
