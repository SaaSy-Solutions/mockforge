/**
 * Angular Adapter
 *
 * Integration adapter for Angular applications
 */

import { ForgeConnect } from '../core/ForgeConnect';
import { ForgeConnectConfig } from '../types';

/**
 * Angular adapter configuration
 */
export interface AngularAdapterConfig extends ForgeConnectConfig {
    /**
     * Only enable in development mode (default: true)
     */
    devOnly?: boolean;
}

/**
 * Angular service for ForgeConnect
 *
 * Provides integration with Angular by:
 * - Only running in development mode
 * - Injectable service for dependency injection
 * - Auto-initialization on service construction
 */
export class AngularForgeConnectService {
    private forgeConnect: ForgeConnect;
    private config: AngularAdapterConfig;
    private initialized: boolean = false;
    private _connected: boolean = false;

    constructor(config: AngularAdapterConfig = {}) {
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
                          (window as any).__NG_DEV_MODE__ === true);

            if (!isDev) {
                return false;
            }
        }

        if (this.initialized) {
            return this.forgeConnect.getConnectionStatus().connected;
        }

        this.initialized = true;
        const connected = await this.forgeConnect.initialize();
        this._connected = connected;
        return connected;
    }

    /**
     * Get the underlying ForgeConnect instance
     */
    getForgeConnect(): ForgeConnect {
        return this.forgeConnect;
    }

    /**
     * Check if connected
     */
    get connected(): boolean {
        return this._connected;
    }

    /**
     * Stop the adapter
     */
    async stop(): Promise<void> {
        await this.forgeConnect.stop();
        this.initialized = false;
        this._connected = false;
    }
}

/**
 * Angular decorator/service setup
 *
 * @example
 * ```typescript
 * // In your Angular module or standalone component
 * import { provideForgeConnect } from '@mockforge/forgeconnect/adapters/angular';
 *
 * @NgModule({
 *   providers: [
 *     provideForgeConnect({ mockMode: 'auto' })
 *   ]
 * })
 * export class AppModule {}
 * ```
 */
export function provideForgeConnect(config?: AngularAdapterConfig) {
    // This would be used with Angular's dependency injection
    // In practice, you'd use Angular's Injectable decorator
    return {
        provide: 'ForgeConnectService',
        useFactory: () => {
            const service = new AngularForgeConnectService(config);
            // Auto-initialize
            service.initialize().catch(err => {
                console.error('[ForgeConnect] Failed to initialize:', err);
            });
            return service;
        },
    };
}

/**
 * Injectable service class for Angular
 *
 * @example
 * ```typescript
 * import { Injectable } from '@angular/core';
 * import { AngularForgeConnectService } from '@mockforge/forgeconnect/adapters/angular';
 *
 * @Injectable({
 *   providedIn: 'root'
 * })
 * export class ForgeConnectService extends AngularForgeConnectService {
 *   constructor() {
 *     super({ mockMode: 'auto' });
 *   }
 * }
 * ```
 */
export class ForgeConnectService extends AngularForgeConnectService {
    constructor(config?: AngularAdapterConfig) {
        super(config);
        // Auto-initialize in constructor
        this.initialize().catch(err => {
            console.error('[ForgeConnect] Failed to initialize:', err);
        });
    }
}
