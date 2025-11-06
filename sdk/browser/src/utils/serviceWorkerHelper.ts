/**
 * Service Worker Helper Utilities
 * 
 * Utilities for registering and managing the ForgeConnect service worker
 */

import { generateServiceWorkerScript } from '../core/ServiceWorkerInterceptor';

/**
 * Register the ForgeConnect service worker script
 * 
 * This function creates a blob URL for the service worker script
 * and registers it. The script is generated dynamically.
 * 
 * @param scope - Service worker scope (default: '/')
 * @returns Promise that resolves to the service worker registration
 */
export async function registerForgeConnectServiceWorker(
    scope: string = '/'
): Promise<ServiceWorkerRegistration> {
    if (!('serviceWorker' in navigator)) {
        throw new Error('Service Workers are not supported in this browser');
    }

    // Generate service worker script
    const script = generateServiceWorkerScript();

    // Create blob URL for the script
    const blob = new Blob([script], { type: 'application/javascript' });
    const url = URL.createObjectURL(blob);

    try {
        // Register the service worker
        const registration = await navigator.serviceWorker.register(url, { scope });

        // Clean up blob URL after registration
        URL.revokeObjectURL(url);

        return registration;
    } catch (error) {
        // Clean up blob URL on error
        URL.revokeObjectURL(url);
        throw error;
    }
}

/**
 * Create a service worker script file for static serving
 * 
 * This generates the service worker script that can be saved to a file
 * and served statically from your server.
 * 
 * @returns Service worker script as string
 */
export function createServiceWorkerFile(): string {
    return generateServiceWorkerScript();
}

