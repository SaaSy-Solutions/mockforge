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
 * Full-featured implementation with RxJS observables
 *
 * @example
 * ```typescript
 * import { Injectable } from '@angular/core';
 * import { BehaviorSubject, Observable } from 'rxjs';
 * import { ForgeConnectService } from '@mockforge/forgeconnect/adapters/angular';
 * import { MockConfig, Environment } from '@mockforge/forgeconnect';
 *
 * @Injectable({
 *   providedIn: 'root'
 * })
 * export class MyForgeConnectService extends ForgeConnectService {
 *   constructor() {
 *     super({ mockMode: 'auto', enableWebSocket: true });
 *   }
 * }
 * ```
 */
export class ForgeConnectService extends AngularForgeConnectService {
    // RxJS observables for reactive state
    // Using any to support both actual RxJS and fallback implementations
    private _connected$: any;
    private _mocks$: any;
    private _environments$: any;
    private _activeEnvironment$: any;
    private _offline$: any;
    private _loading$: any;
    private _error$: any;
    private _liveReloadEnabled$: any;

    // Public observables (read-only)
    public readonly connected$: any;
    public readonly mocks$: any;
    public readonly environments$: any;
    public readonly activeEnvironment$: any;
    public readonly offline$: any;
    public readonly loading$: any;
    public readonly error$: any;
    public readonly liveReloadEnabled$: any;

    constructor(config?: AngularAdapterConfig) {
        super(config);

        // Initialize BehaviorSubject instances
        // Try to use actual RxJS if available, otherwise create simple fallback
        try {
            // @ts-ignore - dynamic import check
            const rxjs = typeof require !== 'undefined' ? require('rxjs') : null;
            const BS = rxjs?.BehaviorSubject || this.createBehaviorSubjectFallback();

            this._connected$ = new BS(false);
            this._mocks$ = new BS([]);
            this._environments$ = new BS([]);
            this._activeEnvironment$ = new BS(null);
            this._offline$ = new BS(false);
            this._loading$ = new BS(false);
            this._error$ = new BS(null);
            this._liveReloadEnabled$ = new BS(true);
        } catch {
            // Fallback if RxJS not available
            this._connected$ = this.createBehaviorSubjectFallback()(false);
            this._mocks$ = this.createBehaviorSubjectFallback()([]);
            this._environments$ = this.createBehaviorSubjectFallback()([]);
            this._activeEnvironment$ = this.createBehaviorSubjectFallback()(null);
            this._offline$ = this.createBehaviorSubjectFallback()(false);
            this._loading$ = this.createBehaviorSubjectFallback()(false);
            this._error$ = this.createBehaviorSubjectFallback()(null);
            this._liveReloadEnabled$ = this.createBehaviorSubjectFallback()(true);
        }

        // Create observables
        this.connected$ = this._connected$.asObservable();
        this.mocks$ = this._mocks$.asObservable();
        this.environments$ = this._environments$.asObservable();
        this.activeEnvironment$ = this._activeEnvironment$.asObservable();
        this.offline$ = this._offline$.asObservable();
        this.loading$ = this._loading$.asObservable();
        this.error$ = this._error$.asObservable();
        this.liveReloadEnabled$ = this._liveReloadEnabled$.asObservable();

        // Auto-initialize in constructor
        this.initialize().catch(err => {
            console.error('[ForgeConnect] Failed to initialize:', err);
            this._error$.next(err instanceof Error ? err.message : 'Unknown error');
        });
    }

    /**
     * Create a simple BehaviorSubject fallback if RxJS is not available
     */
    private createBehaviorSubjectFallback(): new <T>(initialValue: T) => any {
        return class SimpleBehaviorSubject<T> {
            private value: T;
            private listeners: Set<(value: T) => void> = new Set();

            constructor(initialValue: T) {
                this.value = initialValue;
            }

            next(value: T): void {
                this.value = value;
                this.listeners.forEach(listener => listener(value));
            }

            getValue(): T {
                return this.value;
            }

            asObservable(): any {
                return {
                    subscribe: (observer: (value: T) => void) => {
                        this.listeners.add(observer);
                        // Immediately emit current value
                        observer(this.value);
                        return {
                            unsubscribe: () => {
                                this.listeners.delete(observer);
                            }
                        };
                    }
                };
            }
        } as any;
    }

    /**
     * Override initialize to update observables
     */
    async initialize(): Promise<boolean> {
        this._loading$.next(true);
        try {
            const result = await super.initialize();
            this._connected$.next(result);

            if (result) {
                await this.refreshMocks();
                await this.refreshEnvironments();

                // Watch connection status
                const forgeConnect = this.getForgeConnect();
                const status = forgeConnect.getConnectionStatus();
                this._connected$.next(status.connected);
                this._offline$.next(!status.connected);
            }

            return result;
        } catch (err) {
            this._error$.next(err instanceof Error ? err.message : 'Unknown error');
            return false;
        } finally {
            this._loading$.next(false);
        }
    }

    /**
     * Refresh mocks list
     */
    async refreshMocks(): Promise<void> {
        try {
            const forgeConnect = this.getForgeConnect();
            const mocks = await forgeConnect.listMocks();
            this._mocks$.next(mocks);
        } catch (err) {
            console.error('[ForgeConnectService] Failed to refresh mocks:', err);
        }
    }

    /**
     * Refresh environments list
     */
    async refreshEnvironments(): Promise<void> {
        try {
            const forgeConnect = this.getForgeConnect();
            const environments = await forgeConnect.listEnvironments();
            this._environments$.next(environments);
            const active = await forgeConnect.getActiveEnvironment();
            this._activeEnvironment$.next(active);
        } catch (err) {
            console.error('[ForgeConnectService] Failed to refresh environments:', err);
        }
    }

    /**
     * Create a mock from a request
     */
    async createMock(request: any): Promise<any | null> {
        try {
            const forgeConnect = this.getForgeConnect();
            const mock = await forgeConnect.createMockFromRequest(request);
            await this.refreshMocks();
            return mock;
        } catch (err) {
            console.error('[ForgeConnectService] Failed to create mock:', err);
            return null;
        }
    }

    /**
     * Set the active environment
     */
    async setActiveEnvironment(envId: string): Promise<void> {
        try {
            const forgeConnect = this.getForgeConnect();
            await forgeConnect.setActiveEnvironment(envId);
            await this.refreshEnvironments();
        } catch (err) {
            console.error('[ForgeConnectService] Failed to set active environment:', err);
            throw err;
        }
    }

    /**
     * Get current mocks (synchronous)
     */
    getMocks(): any[] {
        return this._mocks$.getValue();
    }

    /**
     * Get current environments (synchronous)
     */
    getEnvironments(): any[] {
        return this._environments$.getValue();
    }

    /**
     * Get active environment (synchronous)
     */
    getActiveEnvironment(): any | null {
        return this._activeEnvironment$.getValue();
    }

    /**
     * Check if connected (synchronous)
     */
    get isConnected(): boolean {
        return this._connected$.getValue();
    }

    /**
     * Check if offline (synchronous)
     */
    get isOffline(): boolean {
        return this._offline$.getValue();
    }

    /**
     * Override stop to clean up observables
     */
    async stop(): Promise<void> {
        await super.stop();
        this._connected$.next(false);
        this._offline$.next(true);
    }
}

// RxJS types - these should be available in Angular projects
// We use type-only imports to avoid runtime dependencies if RxJS isn't available
// In actual Angular projects, these will be provided by @angular/core and rxjs

// Type definitions for RxJS (fallback if not available)
interface BehaviorSubjectLike<T> {
    constructor(initialValue: T): void;
    next(value: T): void;
    getValue(): T;
    asObservable(): ObservableLike<T>;
}

interface ObservableLike<T> {
    subscribe(observer: (value: T) => void): { unsubscribe: () => void };
}

// Try to use actual RxJS types if available, otherwise use fallback
type BehaviorSubjectType<T> = typeof BehaviorSubject extends undefined
    ? BehaviorSubjectLike<T>
    : typeof BehaviorSubject;

type ObservableType<T> = typeof Observable extends undefined
    ? ObservableLike<T>
    : typeof Observable;

// Declare global RxJS types (will be available in Angular projects)
declare const BehaviorSubject: new <T>(initialValue: T) => {
    next(value: T): void;
    getValue(): T;
    asObservable(): ObservableType<T>;
};

declare const Observable: new <T>() => {
    subscribe(observer: (value: T) => void): { unsubscribe: () => void };
};
