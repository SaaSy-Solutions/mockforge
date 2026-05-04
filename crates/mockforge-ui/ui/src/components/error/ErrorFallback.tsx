import { logger } from '@/utils/logger';
import React from 'react';
import { AlertTriangle, RefreshCw } from 'lucide-react';
import { Button } from '../ui/button';

interface ErrorFallbackProps {
  error: Error;
  resetError: () => void;
  title?: string;
  message?: string;
  showDetails?: boolean;
}

export function ErrorFallback({
  error,
  resetError,
  title = "Something went wrong",
  message = "An error occurred while loading this content",
  showDetails = import.meta.env.DEV,
}: ErrorFallbackProps) {
  return (
    <div className="bg-danger-50 border border-danger-200 rounded-lg p-4">
      <div className="flex items-start gap-3">
        <AlertTriangle className="h-5 w-5 text-danger-500 mt-0.5 flex-shrink-0" />
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-medium text-danger-700 mb-1">
            {title}
          </h3>
          <p className="text-sm text-danger-700 mb-3">
            {message}
          </p>

          {showDetails && (
            <details className="mb-3">
              <summary className="text-xs text-danger-600 cursor-pointer hover:text-danger-700">
                Error Details
              </summary>
              <div className="mt-2 p-2 bg-danger-100 border border-danger-300 rounded text-xs font-mono text-danger-900 max-h-32 overflow-y-auto">
                <div className="font-semibold mb-1">Message:</div>
                <div className="whitespace-pre-wrap break-all mb-2">
                  {error.message}
                </div>
                <div className="font-semibold mb-1">Stack:</div>
                <div className="whitespace-pre-wrap break-all">
                  {error.stack}
                </div>
              </div>
            </details>
          )}

          <Button
            onClick={resetError}
            size="sm"
            variant="outline"
            className="text-danger-700 border-danger-300 hover:bg-danger-100"
          >
            <RefreshCw className="h-3 w-3 mr-1" />
            Try Again
          </Button>
        </div>
      </div>
    </div>
  );
}

interface InlineErrorProps {
  error: Error | string;
  className?: string;
}

export function InlineError({ error, className = "" }: InlineErrorProps) {
  const errorMessage = typeof error === 'string' ? error : error.message;

  return (
    <div className={`text-sm text-danger-600 bg-danger-50 border border-danger-200 rounded px-3 py-2 ${className}`}>
      <div className="flex items-center gap-2">
        <AlertTriangle className="h-4 w-4 flex-shrink-0" />
        <span>{errorMessage}</span>
      </div>
    </div>
  );
}
