/**
 * Framework Adapters
 * 
 * Export all framework adapters
 */

export { ReactQueryAdapter, useForgeConnect } from './ReactQueryAdapter';
export type { ReactQueryAdapterConfig } from './ReactQueryAdapter';

export { NextJSAdapter, useNextJSForgeConnect } from './NextJSAdapter';
export type { NextJSAdapterConfig } from './NextJSAdapter';

export { VanillaAdapter, initForgeConnect, autoInitForgeConnect } from './VanillaAdapter';
export type { VanillaAdapterConfig } from './VanillaAdapter';

export { VueAdapter, useForgeConnect as useVueForgeConnect } from './VueAdapter';
export type { VueAdapterConfig } from './VueAdapter';

export { 
    AngularForgeConnectService, 
    ForgeConnectService,
    provideForgeConnect 
} from './AngularAdapter';
export type { AngularAdapterConfig } from './AngularAdapter';

