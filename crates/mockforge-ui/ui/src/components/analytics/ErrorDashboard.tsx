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
          <AlertTriangle className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Error Summary</h3>
        </div>
        <div className="h-64 flex items-center justify-center">
          <div className="animate-pulse text-gray-400">Loading...</div>
        </div>
      </Card>
    );
  }

  if (error || !data?.errors) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <AlertTriangle className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Error Summary</h3>
        </div>
        <div className="h-64 flex items-center justify-center text-gray-400">
          {error ? 'Error loading data' : 'No errors found'}
        </div>
      </Card>
    );
  }

  const categoryColors: Record<string, string> = {
    client_error: 'bg-yellow-100 dark:bg-yellow-900/20 text-yellow-800 dark:text-yellow-200',
    server_error: 'bg-red-100 dark:bg-red-900/20 text-red-800 dark:text-red-200',
    network_error: 'bg-orange-100 dark:bg-orange-900/20 text-orange-800 dark:text-orange-200',
    timeout_error: 'bg-purple-100 dark:bg-purple-900/20 text-purple-800 dark:text-purple-200',
    other: 'bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200',
  };

  const totalErrors = data.errors.reduce((sum, e) => sum + e.count, 0);

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <AlertTriangle className="h-5 w-5 text-red-600 dark:text-red-400" />
          <h3 className="text-lg font-semibold">Error Summary</h3>
        </div>
        <div className="text-sm text-gray-500 dark:text-gray-400">
          {totalErrors.toLocaleString()} total errors
        </div>
      </div>

      {data.errors.length === 0 ? (
        <div className="text-center py-12 text-gray-500 dark:text-gray-400">
          <AlertCircle className="h-12 w-12 mx-auto mb-2 opacity-50" />
          <p>No errors in the selected time range</p>
        </div>
      ) : (
        <div className="space-y-3">
          {data.errors.slice(0, 10).map((err, index) => (
            <div
              key={index}
              className="flex items-start gap-4 p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
            >
              <div className="flex-shrink-0">
                <AlertTriangle className="h-5 w-5 text-red-500" />
              </div>

              <div className="flex-grow">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-medium text-gray-900 dark:text-white">
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

                <div className="text-sm text-gray-600 dark:text-gray-400">
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

                <div className="flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400 mt-1">
                  <Clock className="h-3 w-3" />
                  Last seen: {new Date(err.last_occurrence).toLocaleString()}
                </div>
              </div>

              <div className="flex-shrink-0 text-right">
                <div className="text-2xl font-bold text-gray-900 dark:text-white">
                  {err.count.toLocaleString()}
                </div>
                <div className="text-xs text-gray-500 dark:text-gray-400">
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
