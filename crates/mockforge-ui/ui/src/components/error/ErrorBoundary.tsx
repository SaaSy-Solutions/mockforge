import React, { Component } from 'react';
import type { ReactNode } from 'react';
import { AlertTriangle, RefreshCw, Home } from 'lucide-react';
import { Button } from '../ui/DesignSystem';
import { StatusIcon, ActionIcon } from '../ui/IconSystem';
import { ErrorState } from '../ui/LoadingStates';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
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
    console.error('ErrorBoundary caught an error:', error, errorInfo);
    this.setState({
      error,
      errorInfo,
    });

    // In a production app, you might want to send this to an error reporting service
    // logErrorToService(error, errorInfo);
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
        return this.props.fallback;
      }

      return (
        <div className="min-h-screen bg-background flex items-center justify-center p-4">
          <div className="max-w-lg w-full bg-card border border-gray-200 dark:border-gray-800 rounded-xl shadow-xl p-8 spring-in">
            <div className="text-center">
              <div className="p-4 rounded-full bg-red-50 dark:bg-red-900/20 mb-6 inline-flex">
                <StatusIcon status="error" size="3xl" />
              </div>
              
              <h2 className="text-heading-xl text-primary mb-3">
                Something went wrong
              </h2>
              <p className="text-body-lg text-secondary mb-8">
                An unexpected error occurred in the application. Please try refreshing the page or contact support if the issue persists.
              </p>

              {import.meta.env.DEV && this.state.error && (
                <div className="mb-6 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-left">
                  <div className="text-label-md text-weight-semibold text-danger mb-2">Error Details:</div>
                  <div className="text-mono-sm text-danger whitespace-pre-wrap break-all max-h-32 overflow-y-auto">
                    {this.state.error.message}
                  </div>
                  {this.state.errorInfo && (
                    <div className="mt-3 pt-3 border-t border-red-200 dark:border-red-800">
                      <div className="text-label-sm text-weight-semibold text-danger mb-1">Component Stack:</div>
                      <div className="text-mono-xs text-danger whitespace-pre-wrap break-all max-h-24 overflow-y-auto">
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

              <div className="mt-6 text-body-sm text-tertiary">
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
