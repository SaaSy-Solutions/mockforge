/**
 * Error Reporting Service
 *
 * Provides centralized error reporting with optional Sentry integration.
 *
 * To enable Sentry:
 * 1. Install Sentry: `npm install @sentry/react`
 * 2. Set environment variables:
 *    - VITE_SENTRY_DSN: Your Sentry DSN
 *    - VITE_SENTRY_ENVIRONMENT: Environment name (production, staging, etc.)
 *    - VITE_SENTRY_RELEASE: Release version (optional)
 *
 * Example .env:
 *   VITE_SENTRY_DSN=https://xxx@sentry.io/123
 *   VITE_SENTRY_ENVIRONMENT=production
 */

import { logger } from '@/utils/logger';

// Sentry is optional - dynamically import if DSN is configured
let Sentry: typeof import('@sentry/react') | null = null;

const SENTRY_DSN = import.meta.env.VITE_SENTRY_DSN;
const SENTRY_ENVIRONMENT = import.meta.env.VITE_SENTRY_ENVIRONMENT || 'development';
const SENTRY_RELEASE = import.meta.env.VITE_SENTRY_RELEASE;

/**
 * Initialize error reporting service
 * Call this once at app startup (e.g., in main.tsx)
 */
export async function initErrorReporting(): Promise<void> {
  if (SENTRY_DSN) {
    try {
      Sentry = await import('@sentry/react');
      Sentry.init({
        dsn: SENTRY_DSN,
        environment: SENTRY_ENVIRONMENT,
        release: SENTRY_RELEASE,
        integrations: [
          Sentry.browserTracingIntegration(),
          Sentry.replayIntegration({
            maskAllText: true,
            blockAllMedia: true,
          }),
        ],
        tracesSampleRate: 0.1, // Sample 10% of transactions
        replaysSessionSampleRate: 0.1,
        replaysOnErrorSampleRate: 1.0, // Capture 100% of sessions with errors
      });
      logger.info('[Sentry] Initialized with environment:', SENTRY_ENVIRONMENT);
    } catch (e) {
      logger.warn('[Sentry] Failed to initialize - @sentry/react may not be installed');
    }
  }
}

/**
 * Report an error to the error tracking service
 */
export function reportError(error: Error, context?: Record<string, unknown>): void {
  // Always log in development
  if (import.meta.env.DEV) {
    logger.error('[Error Report]', error, context);
  }

  // Send to Sentry if initialized
  if (Sentry) {
    Sentry.captureException(error, {
      contexts: {
        react: context,
      },
    });
  }
}

/**
 * Report a message to the error tracking service
 */
export function reportMessage(message: string, level: 'info' | 'warning' | 'error' = 'info'): void {
  if (import.meta.env.DEV) {
    logger[level === 'error' ? 'error' : level === 'warning' ? 'warn' : 'info']('[Message Report]', message);
  }

  if (Sentry) {
    Sentry.captureMessage(message, level);
  }
}

/**
 * Set user context for error reports
 */
export function setUser(user: { id: string; email?: string; username?: string } | null): void {
  if (Sentry) {
    Sentry.setUser(user);
  }
}

/**
 * Add breadcrumb for error context
 */
export function addBreadcrumb(breadcrumb: {
  category?: string;
  message: string;
  level?: 'debug' | 'info' | 'warning' | 'error';
  data?: Record<string, unknown>;
}): void {
  if (Sentry) {
    Sentry.addBreadcrumb(breadcrumb);
  }
}

/**
 * Clear any stored errors and user context
 */
export function clearErrors(): void {
  if (Sentry) {
    Sentry.setUser(null);
  }
}

/**
 * Check if Sentry is enabled
 */
export function isSentryEnabled(): boolean {
  return Sentry !== null;
}
