export function reportError(error: Error, context?: any) {
  // Log to console in development
  if (import.meta.env.DEV) {
    console.error('[Error Report]', error, context);
  }

  // In production, this would send to an error tracking service like Sentry
  // Sentry.captureException(error, { contexts: { react: context } });
}

export function clearErrors() {
  // Clear any stored errors if needed
}
