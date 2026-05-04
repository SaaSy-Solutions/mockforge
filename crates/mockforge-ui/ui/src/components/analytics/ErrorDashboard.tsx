/**
 * Error dashboard showing error summary and details
 */

import React from 'react';
import { Card } from '../ui/Card';
import { AlertTriangle, AlertCircle, Clock } from 'lucide-react';
import { useErrorSummary, type AnalyticsFilter } from '@/hooks/useAnalyticsV2';

interface ErrorDashboardProps {
  filter?: AnalyticsFilter;
}

export const ErrorDashboard: React.FC<ErrorDashboardProps> = ({ filter }) => {
  const { data, isLoading, error } = useErrorSummary(filter);

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <AlertTriangle className="h-5 w-5 text-muted-foreground" />
          <h3 className="text-lg font-semibold">Error Summary</h3>
        </div>
        <div className="h-64 flex items-center justify-center">
          <div className="animate-pulse text-muted-foreground">Loading...</div>
        </div>
      </Card>
    );
  }

  if (error || !data?.errors) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <AlertTriangle className="h-5 w-5 text-muted-foreground" />
          <h3 className="text-lg font-semibold">Error Summary</h3>
        </div>
        <div className="h-64 flex items-center justify-center text-muted-foreground">
          {error ? 'Error loading data' : 'No errors found'}
        </div>
      </Card>
    );
  }

  const categoryColors: Record<string, string> = {
    client_error: 'bg-warning-100 dark:bg-warning-900/20 text-warning-700 dark:text-warning-200',
    server_error: 'bg-danger-100 dark:bg-danger-900/20 text-danger-700 dark:text-danger-200',
    network_error: 'bg-orange-100 dark:bg-orange-900/20 text-orange-800 dark:text-orange-200',
    timeout_error: 'bg-purple-100 dark:bg-purple-900/20 text-purple-800 dark:text-purple-200',
    other: 'bg-muted text-foreground',
  };

  const totalErrors = data.errors.reduce((sum, e) => sum + e.count, 0);

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <AlertTriangle className="h-5 w-5 text-danger-600 dark:text-danger-400" />
          <h3 className="text-lg font-semibold">Error Summary</h3>
        </div>
        <div className="text-sm text-muted-foreground">
          {totalErrors.toLocaleString()} total errors
        </div>
      </div>

      {data.errors.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <AlertCircle className="h-12 w-12 mx-auto mb-2 opacity-50" />
          <p>No errors in the selected time range</p>
        </div>
      ) : (
        <div className="space-y-3">
          {data.errors.slice(0, 10).map((err, index) => (
            <div
              key={index}
              className="flex items-start gap-4 p-4 rounded-lg border border-border hover:bg-accent hover:text-accent-foreground/50 transition-colors"
            >
              <div className="flex-shrink-0">
                <AlertTriangle className="h-5 w-5 text-danger-500" />
              </div>

              <div className="flex-grow">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-medium text-foreground">
                    {err.error_type}
                  </span>
                  <span
                    className={`px-2 py-0.5 text-xs rounded-full ${
                      categoryColors[err.error_category] || categoryColors.other
                    }`}
                  >
                    {err.error_category.replace('_', ' ')}
                  </span>
                </div>

                <div className="text-sm text-muted-foreground">
                  <span className="font-semibold">{err.count.toLocaleString()}</span>{' '}
                  occurrences
                  {err.endpoints.length > 0 && (
                    <>
                      {' '}
                      • Affected endpoints:{' '}
                      <span className="font-mono text-xs">
                        {err.endpoints.slice(0, 3).join(', ')}
                        {err.endpoints.length > 3 && ` +${err.endpoints.length - 3} more`}
                      </span>
                    </>
                  )}
                </div>

                <div className="flex items-center gap-1 text-xs text-muted-foreground mt-1">
                  <Clock className="h-3 w-3" />
                  Last seen: {new Date(err.last_occurrence).toLocaleString()}
                </div>
              </div>

              <div className="flex-shrink-0 text-right">
                <div className="text-2xl font-bold text-foreground">
                  {err.count.toLocaleString()}
                </div>
                <div className="text-xs text-muted-foreground">
                  {((err.count / totalErrors) * 100).toFixed(1)}%
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
};
