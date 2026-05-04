import { logger } from '@/utils/logger';
import React, { Component } from 'react';
import type { ReactNode } from 'react';
import { Button } from '../ui/DesignSystem';
import { StatusIcon, ActionIcon } from '../ui/IconSystem';
import { reportError } from '../../services/errorReporting';

interface FallbackProps {
  error: Error;
  resetError: () => void;
}

interface Props {
  children: ReactNode;
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
    logger.error('ErrorBoundary caught an error', error,errorInfo);
    this.setState({
      error,
      errorInfo,
    });

    // Report error to error reporting service
    try {
      reportError(error, errorInfo);
    } catch (e) {
      // Error reporting failed - log but don't crash
      logger.error('Failed to report error',e);
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
        <div className="min-h-screen bg-background flex items-center justify-center p-4" data-testid="error-boundary-fallback">
          <div className="max-w-lg w-full bg-card border border-border rounded-xl shadow-xl p-8 spring-in">
            <div className="text-center">
              <div className="p-4 rounded-full bg-danger-50 dark:bg-danger-900/20 mb-6 inline-flex">
                <StatusIcon status="error" size="3xl" />
              </div>

              <h2 className="text-3xl font-bold text-foreground mb-3">
                Something went wrong
              </h2>
              <p className="text-lg text-muted-foreground mb-8">
                An unexpected error occurred in the application. Please try refreshing the page or contact support if the issue persists.
              </p>

              {import.meta.env.DEV && this.state.error && (
                <div className="mb-6 p-4 bg-danger-50 dark:bg-danger-900/20 border border-danger-200 dark:border-danger-800 rounded-lg text-left">
                  <div className="text-sm font-semibold text-danger-700 dark:text-danger-400 mb-2">Error Details:</div>
                  <div className="text-sm font-mono text-danger-700 dark:text-danger-500 whitespace-pre-wrap break-all max-h-32 overflow-y-auto">
                    {this.state.error.message}
                  </div>
                  {this.state.errorInfo && (
                    <div className="mt-3 pt-3 border-t border-danger-200 dark:border-danger-800">
                      <div className="text-xs font-semibold text-danger-700 dark:text-danger-400 mb-1">Component Stack:</div>
                      <div className="text-xs font-mono text-danger-700 dark:text-danger-500 whitespace-pre-wrap break-all max-h-24 overflow-y-auto">
                        {this.state.errorInfo.componentStack}
                      </div>
                    </div>
                  )}
                </div>
              )}

              <div className="flex gap-3 justify-center">
                <Button
                  onClick={this.handleRetry}
                  className="flex items-center gap-2 spring-hover"
                  variant="primary"
                >
                  <ActionIcon action="view" />
                  Try Again
                </Button>
                <Button
                  onClick={this.handleGoHome}
                  variant="secondary"
                  className="flex items-center gap-2 spring-hover"
                >
                  <ActionIcon action="view" />
                  Go Home
                </Button>
              </div>

              <div className="mt-6 text-sm text-muted-foreground">
                If this problem persists, please contact support with the error details above.
              </div>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
