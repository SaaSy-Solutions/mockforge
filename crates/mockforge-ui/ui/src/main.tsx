import { logger } from '@/utils/logger';
import { StrictMode, lazy, Suspense } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import '@xyflow/react/dist/style.css'
import './index.css'
import App from './App.tsx'
import { useThemePaletteStore } from './stores/useThemePaletteStore'
import { registerServiceWorker, unregisterServiceWorker } from './utils/serviceWorker'
import { I18nProvider } from './i18n/I18nProvider'
import { initErrorReporting } from './services/errorReporting'

// Lazy load React Query DevTools only in development
const ReactQueryDevtools = import.meta.env.DEV
  ? lazy(() =>
    import('@tanstack/react-query-devtools').then((m) => ({
      default: m.ReactQueryDevtools,
    }))
  )
  : null;

// Initialize theme store
useThemePaletteStore.getState().init();
void initErrorReporting();

// Always unregister service workers in dev to prevent stale cache issues
if (import.meta.env.DEV) {
  unregisterServiceWorker();
}

// Register service worker for PWA functionality
if (import.meta.env.PROD) {
  registerServiceWorker({
    onSuccess: (registration) => {
      console.log('[PWA] Service worker registered successfully');
      logger.info('PWA: Service worker registered', { registration });
    },
    onUpdate: (registration) => {
      console.log('[PWA] New service worker available');
      logger.info('PWA: New service worker available', { registration });
      // Optionally show update notification to user
      if (window.confirm('New version available! Reload to update?')) {
        registration.waiting?.postMessage({ type: 'SKIP_WAITING' });
        window.location.reload();
      }
    },
  });
}

// Create a client with optimized defaults for performance
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: (failureCount, error: Error & { status?: number }) => {
        // Don't retry on 4xx errors (client errors)
        if (error?.status && error.status >= 400 && error.status < 500) {
          return false;
        }
        // Retry up to 3 times for network/server errors
        return failureCount < 3;
      },
      retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
      staleTime: 30000, // 30 seconds - data considered fresh
      gcTime: 10 * 60 * 1000, // 10 minutes - keep in cache longer
      refetchOnWindowFocus: false, // Disable to reduce network requests
      refetchOnReconnect: true, // Refetch when connection restored
      refetchOnMount: true, // Always refetch on mount for fresh data
      networkMode: 'online', // Only run queries when online
    },
    mutations: {
      retry: (failureCount, error: Error & { status?: number }) => {
        // Don't retry mutations on client errors
        if (error?.status && error.status >= 400 && error.status < 500) {
          return false;
        }
        return failureCount < 2; // Retry mutations once
      },
      retryDelay: 1000,
      networkMode: 'online',
    },
  },
});

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <I18nProvider>
        <App />
        {ReactQueryDevtools && (
          <Suspense fallback={null}>
            <ReactQueryDevtools initialIsOpen={false} />
          </Suspense>
        )}
      </I18nProvider>
    </QueryClientProvider>
  </StrictMode>,
)
