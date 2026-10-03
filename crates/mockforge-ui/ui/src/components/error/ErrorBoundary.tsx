import { logger } from '@/utils/logger';
import React, { Component } from 'react';
import type { ReactNode } from 'react';
import { Button } from '../ui/DesignSystem';
import { AlertTriangle, Home, RotateCcw } from 'lucide-react';
import { reportError } from '../../services/errorReporting';

interface FallbackProps {
  error: Error;
  resetError: () => void;
}

interface Props {
  children: ReactNode;
  resetKey?: string;
  fallback?: ReactNode | ((props: FallbackProps) => ReactNode);
}

interface State {
  hasError: boolean;
  error?: Error;
  errorInfo?: React.ErrorInfo;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    logger.error('ErrorBoundary caught an error', error, { componentStack: errorInfo.componentStack });
    this.setState({
      error,
      errorInfo,
    });

    // Report error to error reporting service
    try {
      reportError(error, { componentStack: errorInfo.componentStack });
    } catch (e) {
      // Error reporting failed - log but don't crash
      logger.error('Failed to report error',e);
    }
  }

  componentDidUpdate(previous: Props) {
    if (previous.resetKey !== this.props.resetKey && this.state.hasError) {
      this.handleRetry();
    }
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: undefined, errorInfo: undefined });
  };

  handleGoHome = () => {
    window.location.href = '/';
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        // Support both ReactNode and function fallbacks
        if (typeof this.props.fallback === 'function') {
          return this.props.fallback({
            error: this.state.error!,
            resetError: this.handleRetry,
          });
        }
        return this.props.fallback;
      }

      return (
        <div className="flex min-h-[60vh] items-center justify-center p-4" data-testid="error-boundary-fallback">
          <div className="w-full max-w-lg rounded-xl border border-border bg-card p-6 shadow-sm">
            <div className="flex items-start gap-3">
              <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-danger-50 text-danger-600 dark:bg-danger-900/20 dark:text-danger-400">
                <AlertTriangle className="h-5 w-5" aria-hidden />
              </div>
              <div className="min-w-0">
                <h2 className="text-base font-semibold text-foreground">
                  Something went wrong
                </h2>
                <p className="mt-1 text-sm text-muted-foreground">
                  This page hit an unexpected error. Try again, or head back to the dashboard. If it keeps happening, contact support.
                </p>
              </div>
            </div>

            {import.meta.env.DEV && this.state.error && (
              <details open className="mt-4 rounded-lg border border-border bg-bg-secondary text-left">
                <summary className="cursor-pointer px-3 py-2 text-xs font-medium text-muted-foreground">
                  Error details (development only)
                </summary>
                <div className="border-t border-border px-3 py-2">
                  <div className="max-h-32 overflow-y-auto whitespace-pre-wrap break-all font-mono text-xs text-danger-700 dark:text-danger-400">
                    {this.state.error.message}
                  </div>
                  {this.state.errorInfo && (
                    <div className="mt-2 border-t border-border pt-2">
                      <div className="mb-1 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Component stack</div>
                      <div className="max-h-24 overflow-y-auto whitespace-pre-wrap break-all font-mono text-[11px] text-muted-foreground">
                        {this.state.errorInfo.componentStack}
                      </div>
                    </div>
                  )}
                </div>
              </details>
            )}

            <div className="mt-5 flex justify-end gap-2">
              <Button onClick={this.handleGoHome} variant="outline" size="sm">
                <Home className="mr-2 h-4 w-4" aria-hidden />
                Go to dashboard
              </Button>
              <Button onClick={this.handleRetry} variant="primary" size="sm">
                <RotateCcw className="mr-2 h-4 w-4" aria-hidden />
                Try Again
              </Button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
