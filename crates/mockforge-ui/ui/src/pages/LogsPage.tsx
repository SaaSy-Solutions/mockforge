import React, { useState, useMemo } from 'react';
import { FileText, Search, Download, RefreshCw, ChevronDown } from 'lucide-react';
import { useLogs } from '../hooks/useApi';
import type { RequestLog } from '../types';
import {
  PageHeader,
  ModernCard,
  ModernBadge,
  Alert,
  EmptyState,
  Section
} from '../components/ui/DesignSystem';
import { Input } from '../components/ui/input';
import { Button } from '../components/ui/button';

type StatusFilter = 'all' | '2xx' | '4xx' | '5xx';
type MethodFilter = 'ALL' | 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS';

const methodColors = {
  GET: 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400',
  POST: 'bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-400',
  PUT: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400',
  DELETE: 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400',
  PATCH: 'bg-purple-100 text-purple-800 dark:bg-purple-900/20 dark:text-purple-400',
  HEAD: 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400',
  OPTIONS: 'bg-indigo-100 text-indigo-800 dark:bg-indigo-900/20 dark:text-indigo-400',
};


function getStatusBadge(statusCode: number): 'success' | 'warning' | 'error' | 'info' {
  if (statusCode >= 200 && statusCode < 300) return 'success';
  if (statusCode >= 400 && statusCode < 500) return 'warning';
  if (statusCode >= 500) return 'error';
  return 'info';
}

function formatTimestamp(timestamp: string): string {
  const date = new Date(timestamp);
  const formatted = date.toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    timeZoneName: 'short'
  });
  return formatted;
}

export function LogsPage() {
  const [searchTerm, setSearchTerm] = useState('');
  const [methodFilter, setMethodFilter] = useState<MethodFilter>('ALL');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [limit, setLimit] = useState(100);
  const [displayLimit, setDisplayLimit] = useState(50);

  const { data: logs, isLoading, error, refetch } = useLogs({
    method: methodFilter === 'ALL' ? undefined : methodFilter,
    path: searchTerm || undefined,
    limit,
  });

  // Reset display limit when filters change
  React.useEffect(() => {
    setDisplayLimit(50);
  }, [searchTerm, methodFilter, statusFilter, limit]);

  const filteredLogs = useMemo(() => {
    if (!logs) return [];

    let filtered = logs;

    // Apply status filter client-side
    if (statusFilter !== 'all') {
      const start = statusFilter === '2xx' ? 200 : statusFilter === '4xx' ? 400 : 500;
      const end = start + 99;
      filtered = filtered.filter((log: RequestLog) => log.status_code >= start && log.status_code <= end);
    }

    // Apply display limit for progressive loading
    return filtered.slice(0, displayLimit);
  }, [logs, displayLimit, statusFilter]);

  const hasMoreToShow = useMemo(() => {
    if (!logs) return false;
    let filtered = logs;
    if (statusFilter !== 'all') {
      const start = statusFilter === '2xx' ? 200 : statusFilter === '4xx' ? 400 : 500;
      const end = start + 99;
      filtered = filtered.filter((log: RequestLog) => log.status_code >= start && log.status_code <= end);
    }
    return filteredLogs.length < filtered.length;
  }, [logs, filteredLogs.length, statusFilter]);

  const handleExport = () => {
    if (!filteredLogs.length) return;

    try {
      const csvContent = [
        ['Timestamp', 'Method', 'Path', 'Status Code', 'Response Time (ms)', 'Client IP', 'User Agent'].join(','),
        ...filteredLogs.map((log: RequestLog) => [
          log.timestamp,
          log.method,
          `"${log.path}"`,
          log.status_code,
          log.response_time_ms,
          log.client_ip || '',
          `"${log.user_agent || ''}"`
        ].join(','))
      ].join('\n');

      const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
      const link = document.createElement('a');
      const url = URL.createObjectURL(blob);
      link.setAttribute('href', url);
      link.setAttribute('download', `mockforge-logs-${new Date().toISOString().split('T')[0]}.csv`);
      link.style.visibility = 'hidden';
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Failed to export logs:', err);
      alert('Failed to export logs. Please try again.');
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Request Logs"
          subtitle="Monitor and analyze API requests"
        />
        <EmptyState
          icon={<FileText className="h-12 w-12" />}
          title="Loading logs..."
          description="Fetching recent request data from the server."
        />
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Request Logs"
          subtitle="Monitor and analyze API requests"
        />
        <Alert
          type="error"
          title="Failed to load logs"
          message={error instanceof Error ? error.message : 'Unable to fetch request logs. Please try again.'}
        />
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <PageHeader
        title="Request Logs"
        subtitle="Monitor and analyze API requests in real-time"
        action={
          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              onClick={() => refetch()}
              className="flex items-center gap-2"
            >
              <RefreshCw className="h-4 w-4" />
              Refresh
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleExport}
              disabled={!filteredLogs.length}
              className="flex items-center gap-2"
            >
              <Download className="h-4 w-4" />
              Export CSV
            </Button>
          </div>
        }
      />

      <Section
        title="Filters & Search"
        subtitle="Refine your log view with advanced filtering options"
      >
        <ModernCard>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {/* Search Input */}
            <div className="space-y-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Search Path
              </label>
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                <Input
                  placeholder="Filter by path..."
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="pl-10"
                />
              </div>
            </div>

            {/* Method Filter */}
            <div className="space-y-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                HTTP Method
              </label>
              <select
                value={methodFilter}
                onChange={(e) => setMethodFilter(e.target.value as MethodFilter)}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                <option value="ALL">All Methods</option>
                <option value="GET">GET</option>
                <option value="POST">POST</option>
                <option value="PUT">PUT</option>
                <option value="DELETE">DELETE</option>
                <option value="PATCH">PATCH</option>
                <option value="HEAD">HEAD</option>
                <option value="OPTIONS">OPTIONS</option>
              </select>
            </div>

            {/* Status Filter */}
            <div className="space-y-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Status Code
              </label>
              <select
                value={statusFilter}
                onChange={(e) => setStatusFilter(e.target.value as StatusFilter)}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                <option value="all">All Status</option>
                <option value="2xx">2xx Success</option>
                <option value="4xx">4xx Client Error</option>
                <option value="5xx">5xx Server Error</option>
              </select>
            </div>

            {/* Fetch Limit */}
            <div className="space-y-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Fetch Limit
              </label>
              <select
                value={limit}
                onChange={(e) => setLimit(Number(e.target.value))}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                <option value={50}>50</option>
                <option value={100}>100</option>
                <option value={250}>250</option>
                <option value={500}>500</option>
                <option value={1000}>1000</option>
              </select>
            </div>
          </div>
        </ModernCard>
      </Section>

      <Section
        title={`Request Logs (${filteredLogs.length})`}
        subtitle={`Showing ${filteredLogs.length} of ${logs?.length || 0} loaded requests${hasMoreToShow ? ' • More available' : ''}`}
      >
        <ModernCard>
          {filteredLogs.length === 0 ? (
            <EmptyState
              icon={<FileText className="h-12 w-12" />}
              title="No logs found"
              description={
                logs?.length === 0
                  ? "No request logs are available. Make some API calls to see data here."
                  : "No logs match your current filters. Try adjusting your search criteria."
              }
            />
          ) : (
            <div className="space-y-4">
              <div className="space-y-2 max-h-96 overflow-y-auto">
                {filteredLogs.map((log: RequestLog) => (
                <div
                  key={log.id}
                  className="flex items-center justify-between p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
                >
                  <div className="flex items-center gap-4">
                    {/* Method Badge */}
                    <div className={`px-3 py-1 rounded-full text-xs font-semibold ${methodColors[log.method as keyof typeof methodColors] || 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400'}`}>
                      {log.method}
                    </div>

                    {/* Path and Timestamp */}
                    <div className="min-w-0 flex-1">
                      <div className="font-mono text-sm text-gray-900 dark:text-gray-100 truncate">
                        {log.path}
                      </div>
                      <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        {formatTimestamp(log.timestamp)}
                        {log.client_ip && (
                          <span className="ml-2">• {log.client_ip}</span>
                        )}
                      </div>
                      {log.user_agent && (
                        <div className="text-xs text-gray-400 dark:text-gray-500 mt-1 truncate">
                          {log.user_agent}
                        </div>
                      )}
                    </div>
                  </div>

                  <div className="flex items-center gap-4">
                    {/* Response Time */}
                    <div className="text-right">
                      <div className="text-sm font-medium text-gray-900 dark:text-gray-100">
                        {log.response_time_ms}ms
                      </div>
                      <div className="text-xs text-gray-500 dark:text-gray-400">
                        Response Time
                      </div>
                    </div>

                    {/* Status Badge */}
                    <ModernBadge
                      variant={getStatusBadge(log.status_code)}
                      size="sm"
                    >
                      {log.status_code}
                    </ModernBadge>
                  </div>
                </div>
              ))}
              </div>

              {/* Load More Button */}
              {hasMoreToShow && (
                <div className="flex justify-center pt-4 border-t border-gray-200 dark:border-gray-700">
                  <Button
                    variant="outline"
                    onClick={() => setDisplayLimit(prev => prev + 50)}
                    className="flex items-center gap-2"
                  >
                    <ChevronDown className="h-4 w-4" />
                    Show more logs ({(logs?.length || 0) - filteredLogs.length} remaining)
                  </Button>
                </div>
              )}
            </div>
          )}
        </ModernCard>
      </Section>
    </div>
  );
}
