/**
 * ForgeConnect - Browser SDK for MockForge
 *
 * @packageDocumentation
 */

export { ForgeConnect } from './core/ForgeConnect';
export { MockForgeClient } from './core/MockForgeClient';
export { RequestInterceptor } from './core/RequestInterceptor';
export { ServiceWorkerInterceptor, generateServiceWorkerScript } from './core/ServiceWorkerInterceptor';
export { WebSocketClient } from './core/WebSocketClient';
export { registerForgeConnectServiceWorker, createServiceWorkerFile } from './utils/serviceWorkerHelper';

export type {
    ForgeConnectConfig,
    MockConfig,
    MockResponse,
    CapturedRequest,
    ConnectionStatus,
    Environment,
    EnvironmentVariable,
} from './types';

// Export adapters
export { VueAdapter, useForgeConnect as useForgeConnectVue } from './adapters/VueAdapter';
export { AngularForgeConnectService, ForgeConnectService, provideForgeConnect } from './adapters/AngularAdapter';
export { ReactQueryAdapter, useForgeConnect as useForgeConnectReact } from './adapters/ReactQueryAdapter';
export { EnvironmentManager } from './core/EnvironmentManager';

// Default export
export { ForgeConnect as default } from './core/ForgeConnect';
