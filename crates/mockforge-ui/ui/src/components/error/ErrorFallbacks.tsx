import { logger } from '@/utils/logger';
import React from 'react';
import { AlertTriangle, RefreshCw, Home, Database, Clock } from 'lucide-react';
import { Button } from '../ui/button';
import { StatusIcon } from '../ui/IconSystem';

export interface ErrorFallbackProps {
  error?: Error;
  resetError?: () => void;
  retry?: () => Promise<void> | void;
  title?: string;
  description?: string;
  showDetails?: boolean;
  showHomeButton?: boolean;
  className?: string;
}

// Generic error fallback for any component
export function ErrorFallback({
  error,
  resetError,
  retry,
  title = 'Something went wrong',
  description = 'An unexpected error occurred. Please try again.',
  showDetails = false,
  showHomeButton = false,
  className = ''
}: ErrorFallbackProps) {
  return (
    <div className={`flex flex-col items-center justify-center p-8 text-center ${className}`}>
      <div className="p-4 rounded-full bg-red-50 dark:bg-red-900/20 mb-4">
        <StatusIcon status="error" size="lg" />
      </div>

      <h3 className="text-xl font-bold text-gray-900 dark:text-gray-100 mb-2">{title}</h3>
      <p className="text-base text-gray-600 dark:text-gray-400 mb-6 max-w-md">{description}</p>

      {showDetails && error && (
        <details className="mb-6 w-full max-w-md">
          <summary className="cursor-pointer text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:text-gray-100 mb-2">
            Show error details
          </summary>
          <div className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-left">
            <div className="text-sm font-mono text-red-700 dark:text-red-500 break-all">
              {error.message}
            </div>
          </div>
        </details>
      )}

      <div className="flex gap-3">
        {retry && (
          <Button
            onClick={retry}
            className="flex items-center gap-2"
            variant="default"
          >
            <RefreshCw className="h-4 w-4" />
            Try again
          </Button>
        )}
        {resetError && (
          <Button
            onClick={resetError}
            variant="outline"
            className="flex items-center gap-2"
          >
            <RefreshCw className="h-4 w-4" />
            Reset
          </Button>
        )}
        {showHomeButton && (
          <Button
            onClick={() => window.location.href = '/'}
            variant="outline"
            className="flex items-center gap-2"
          >
            <Home className="h-4 w-4" />
            Go home
          </Button>
        )}
      </div>
    </div>
  );
}

// Specific error fallbacks for common scenarios
export function NetworkErrorFallback({ retry, resetError }: { retry?: () => void; resetError?: () => void }) {
  return (
    <ErrorFallback
      title="Connection Problem"
      description="Unable to connect to the server. Please check your internet connection and try again."
      retry={retry}
      resetError={resetError}
      className="min-h-[200px]"
    />
  );
}

export function DataErrorFallback({ retry, resetError }: { retry?: () => void; resetError?: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center p-8 text-center">
      <div className="p-4 rounded-full bg-orange-50 dark:bg-orange-900/20 mb-4">
        <Database className="h-8 w-8 text-orange-600 dark:text-orange-400" />
      </div>

      <h3 className="text-xl font-bold text-gray-900 dark:text-gray-100 mb-2">Data Loading Failed</h3>
      <p className="text-base text-gray-600 dark:text-gray-400 mb-6 max-w-md">
        There was a problem loading the data. This might be a temporary issue.
      </p>

      <div className="flex gap-3">
        {retry && (
          <Button
            onClick={retry}
            className="flex items-center gap-2"
            variant="default"
          >
            <RefreshCw className="h-4 w-4" />
            Reload data
          </Button>
        )}
        {resetError && (
          <Button
            onClick={resetError}
            variant="outline"
            className="flex items-center gap-2"
          >
            <Home className="h-4 w-4" />
            Reset view
          </Button>
        )}
      </div>
    </div>
  );
}

export function TimeoutErrorFallback({ retry, resetError }: { retry?: () => void; resetError?: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center p-8 text-center">
      <div className="p-4 rounded-full bg-yellow-50 dark:bg-yellow-900/20 mb-4">
        <Clock className="h-8 w-8 text-yellow-600 dark:text-yellow-400" />
      </div>

      <h3 className="text-xl font-bold text-gray-900 dark:text-gray-100 mb-2">Request Timed Out</h3>
      <p className="text-base text-gray-600 dark:text-gray-400 mb-6 max-w-md">
        The request took too long to complete. This might be due to a slow connection or server issues.
      </p>

      <div className="flex gap-3">
        {retry && (
          <Button
            onClick={retry}
            className="flex items-center gap-2"
            variant="default"
          >
            <RefreshCw className="h-4 w-4" />
            Try again
          </Button>
        )}
        {resetError && (
          <Button
            onClick={resetError}
            variant="outline"
            className="flex items-center gap-2"
          >
            <Home className="h-4 w-4" />
            Reset
          </Button>
        )}
      </div>
    </div>
  );
}

// Compact error fallback for smaller components
export function CompactErrorFallback({
  error,
  retry,
  message = 'Failed to load'
}: {
  error?: Error;
  retry?: () => void;
  message?: string;
}) {
  return (
    <div className="flex items-center justify-between p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
      <div className="flex items-center gap-3">
        <AlertTriangle className="h-5 w-5 text-red-600 dark:text-red-400" />
        <div>
          <div className="text-sm font-medium text-red-700 dark:text-red-500">{message}</div>
          {error && (
            <div className="text-sm text-gray-600 dark:text-gray-400">{error.message}</div>
          )}
        </div>
      </div>

      {retry && (
        <Button
          onClick={retry}
          size="sm"
          variant="outline"
          className="flex items-center gap-2"
        >
          <RefreshCw className="h-3 w-3" />
          Retry
        </Button>
      )}
    </div>
  );
}

// Lightweight error boundary wrapper
class ErrorBoundaryWrapper extends React.Component<
  { children: React.ReactNode; fallback?: React.ComponentType<ErrorFallbackProps> },
  { hasError: boolean; error?: Error }
> {
  constructor(props: { children: React.ReactNode; fallback?: React.ComponentType<ErrorFallbackProps> }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    logger.error('Component error caught', error,errorInfo);
  }

  render() {
    if (this.state.hasError) {
      const FallbackComponent = this.props.fallback || ErrorFallback;
      return (
        <FallbackComponent
          error={this.state.error}
          resetError={() => this.setState({ hasError: false, error: undefined })}
        />
      );
    }

    return this.props.children;
  }
}

// HOC for wrapping components with error boundaries
export function withErrorBoundary<P extends object>(
  Component: React.ComponentType<P>,
  fallback?: React.ComponentType<ErrorFallbackProps>
) {
  const WrappedComponent = (props: P) => {
    return (
      <ErrorBoundaryWrapper fallback={fallback}>
        <Component {...props} />
      </ErrorBoundaryWrapper>
    );
  };

  WrappedComponent.displayName = `withErrorBoundary(${Component.displayName || Component.name})`;
  return WrappedComponent;
}
