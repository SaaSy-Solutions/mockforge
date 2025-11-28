/**
 * Service Worker Registration Utility
 * Handles registration and updates of the service worker for PWA functionality
 */

const isLocalhost = Boolean(
  window.location.hostname === 'localhost' ||
  window.location.hostname === '[::1]' ||
  window.location.hostname.match(/^127(?:\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)){3}$/)
);

type Config = {
  onSuccess?: (registration: ServiceWorkerRegistration) => void;
  onUpdate?: (registration: ServiceWorkerRegistration) => void;
};

const SW_VERSION = 'v3';

export function registerServiceWorker(config?: Config) {
  if (!('serviceWorker' in navigator)) return;

  const publicUrl = new URL(import.meta.env.BASE_URL || '/', window.location.href);
  if (publicUrl.origin !== window.location.origin) {
    // Service worker won't work if PUBLIC_URL is on a different origin
    return;
  }

  // Append a version to the SW URL so caches are busted when we roll a new build
  const swUrl = `${import.meta.env.BASE_URL || '/'}sw.js?version=${SW_VERSION}`;

  // Proactively clear old registrations/caches when version changes
  const clearStaleServiceWorkers = async () => {
    const registrations = await navigator.serviceWorker.getRegistrations();
    await Promise.all(
      registrations.map(async (registration) => {
        const url = registration.active?.scriptURL || registration.installing?.scriptURL || registration.waiting?.scriptURL;
        if (url && !url.includes(`version=${SW_VERSION}`)) {
          try {
            await registration.unregister();
          } catch (err) {
            console.warn('[Service Worker] Failed to unregister stale registration', err);
          }
        }
      })
    );

    // Also clear all caches if they do not include the current version
    const cacheNames = await caches.keys();
    await Promise.all(
      cacheNames.map((name) => {
        if (!name.includes(SW_VERSION)) {
          return caches.delete(name);
        }
        return Promise.resolve(false);
      })
    );
  };

  window.addEventListener('load', () => {
    clearStaleServiceWorkers().catch((err) => {
      console.warn('[Service Worker] Failed to clear stale registrations', err);
    });

    if (isLocalhost) {
      // Running on localhost - check if service worker exists
      checkValidServiceWorker(swUrl, config);

      navigator.serviceWorker.ready.then(() => {
        console.log('[Service Worker] Ready on localhost');
      });
    } else {
      // Production - register service worker
      registerValidSW(swUrl, config);
    }
  });
}

function registerValidSW(swUrl: string, config?: Config) {
  navigator.serviceWorker
    .register(swUrl)
    .then((registration) => {
      registration.onupdatefound = () => {
        const installingWorker = registration.installing;
        if (installingWorker == null) {
          return;
        }
        installingWorker.onstatechange = () => {
          if (installingWorker.state === 'installed') {
            if (navigator.serviceWorker.controller) {
              // New content available - show update notification
              console.log('[Service Worker] New content available; please refresh.');
              if (config && config.onUpdate) {
                config.onUpdate(registration);
              } else {
                // If no custom handler, activate the new worker immediately and reload
                registration.waiting?.postMessage({ type: 'SKIP_WAITING' });
                window.location.reload();
              }
            } else {
              // Content cached for offline use
              console.log('[Service Worker] Content cached for offline use.');
              if (config && config.onSuccess) {
                config.onSuccess(registration);
              }
            }
          }
        };
      };
    })
    .catch((error) => {
      console.error('[Service Worker] Registration failed:', error);
    });
}

function checkValidServiceWorker(swUrl: string, config?: Config) {
  // Check if the service worker can be found
  fetch(swUrl, {
    headers: { 'Service-Worker': 'script' },
  })
    .then((response) => {
      // Ensure service worker exists, and that we really are getting a JS file
      const contentType = response.headers.get('content-type');
      if (
        response.status === 404 ||
        (contentType != null && contentType.indexOf('javascript') === -1)
      ) {
        // No service worker found - unregister
        navigator.serviceWorker.ready.then((registration) => {
          registration.unregister();
        });
      } else {
        // Service worker found - proceed with registration
        registerValidSW(swUrl, config);
      }
    })
    .catch(() => {
      console.log('[Service Worker] No internet connection found. App is running in offline mode.');
    });
}

export function unregisterServiceWorker() {
  if ('serviceWorker' in navigator) {
    navigator.serviceWorker.ready
      .then((registration) => {
        registration.unregister();
      })
      .catch((error) => {
        console.error(error.message);
      });
  }
}

/**
 * Check if app is running in offline mode
 */
export function isOffline(): boolean {
  return !navigator.onLine;
}

/**
 * Listen for online/offline status changes
 */
export function onOnlineStatusChange(callback: (isOnline: boolean) => void) {
  window.addEventListener('online', () => callback(true));
  window.addEventListener('offline', () => callback(false));
}
