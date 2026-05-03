import { logger } from '@/utils/logger';
import React, { useState, useMemo } from 'react';
import { FileText, Search, Download, RefreshCw, ChevronDown, Eye } from 'lucide-react';
import { useLogs } from '../hooks/useApi';
import { usePreferencesStore } from '../stores/usePreferencesStore';
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
import { ResponseTraceModal } from '../components/reality/ResponseTraceModal';

type StatusFilter = 'all' | '2xx' | '4xx' | '5xx';
type MethodFilter = 'ALL' | 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS';

const methodColors = {
  GET: 'bg-success-100 text-success-700 dark:bg-success-900/20 dark:text-success-400',
  POST: 'bg-info-100 text-info-700 dark:bg-info-900/20 dark:text-info-400',
  PUT: 'bg-warning-100 text-warning-700 dark:bg-warning-900/20 dark:text-warning-400',
  DELETE: 'bg-danger-100 text-danger-700 dark:bg-danger-900/20 dark:text-danger-400',
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
  const logPrefs = usePreferencesStore((s) => s.preferences.logs);
  const [searchTerm, setSearchTerm] = useState('');
  const [methodFilter, setMethodFilter] = useState<MethodFilter>('ALL');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [limit, setLimit] = useState(100);
  // `itemsPerPage` is the initial display slice; users can still "load more".
  const [displayLimit, setDisplayLimit] = useState(logPrefs.itemsPerPage);
  const [selectedTraceRequestId, setSelectedTraceRequestId] = useState<string | null>(null);

  const { data: logs, isLoading, error, refetch } = useLogs({
    method: methodFilter === 'ALL' ? undefined : methodFilter,
    path: searchTerm || undefined,
    limit,
  });

  // Reset display limit when filters or the preference change.
  React.useEffect(() => {
    setDisplayLimit(logPrefs.itemsPerPage);
  }, [searchTerm, methodFilter, statusFilter, limit, logPrefs.itemsPerPage]);

  const filteredLogs = useMemo(() => {
    if (!logs) return [];

    let filtered = logs;

    // Apply status filter client-side
    if (statusFilter !== 'all') {
      const start = statusFilter === '2xx' ? 200 : statusFilter === '4xx' ? 400 : 500;
      const end = start + 99;
      filtered = filtered.filter((log: RequestLog) => log.status_code >= start && log.status_code <= end);
    }

    // Apply user's default time-range window (hours).
    if (logPrefs.defaultTimeRange > 0) {
      const cutoff = Date.now() - logPrefs.defaultTimeRange * 3600 * 1000;
      filtered = filtered.filter((log: RequestLog) => {
        const ts = Date.parse(log.timestamp);
        return Number.isFinite(ts) ? ts >= cutoff : true;
      });
    }

    // Apply display limit for progressive loading
    return filtered.slice(0, displayLimit);
  }, [logs, displayLimit, statusFilter, logPrefs.defaultTimeRange]);

  const hasMoreToShow = useMemo(() => {
    if (!logs) return false;
    let filtered = logs;
    if (statusFilter !== 'all') {
      const start = statusFilter === '2xx' ? 200 : statusFilter === '4xx' ? 400 : 500;
      const end = start + 99;
      filtered = filtered.filter((log: RequestLog) => log.status_code >= start && log.status_code <= end);
    }
    if (logPrefs.defaultTimeRange > 0) {
      const cutoff = Date.now() - logPrefs.defaultTimeRange * 3600 * 1000;
      filtered = filtered.filter((log: RequestLog) => {
        const ts = Date.parse(log.timestamp);
        return Number.isFinite(ts) ? ts >= cutoff : true;
      });
    }
    return filteredLogs.length < filtered.length;
  }, [logs, filteredLogs.length, statusFilter, logPrefs.defaultTimeRange]);

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
      logger.error('Failed to export logs',err);
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
              <label className="text-sm font-medium text-foreground">
                Search Path
              </label>
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
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
              <label className="text-sm font-medium text-foreground">
                HTTP Method
              </label>
              <select
                value={methodFilter}
                onChange={(e) => setMethodFilter(e.target.value as MethodFilter)}
                className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground focus:ring-2 focus:ring-blue-500 focus:border-transparent"
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
              <label className="text-sm font-medium text-foreground">
                Status Code
              </label>
              <select
                value={statusFilter}
                onChange={(e) => setStatusFilter(e.target.value as StatusFilter)}
                className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                <option value="all">All Status</option>
                <option value="2xx">2xx Success</option>
                <option value="4xx">4xx Client Error</option>
                <option value="5xx">5xx Server Error</option>
              </select>
            </div>

            {/* Fetch Limit */}
            <div className="space-y-2">
              <label className="text-sm font-medium text-foreground">
                Fetch Limit
              </label>
              <select
                value={limit}
                onChange={(e) => setLimit(Number(e.target.value))}
                className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground focus:ring-2 focus:ring-blue-500 focus:border-transparent"
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
                  className={`flex items-center justify-between rounded-lg border border-border hover:bg-accent hover:text-accent-foreground/50 transition-colors ${
                    logPrefs.compactView ? 'p-2' : 'p-4'
                  }`}
                >
                  <div className={`flex items-center ${logPrefs.compactView ? 'gap-2' : 'gap-4'}`}>
                    {/* Method Badge */}
                    <div className={`px-3 py-1 rounded-full text-xs font-semibold ${methodColors[log.method as keyof typeof methodColors] || 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400'}`}>
                      {log.method}
                    </div>

                    {/* Path and Timestamp */}
                    <div className="min-w-0 flex-1">
                      <div className="font-mono text-sm text-foreground truncate">
                        {log.path}
                      </div>
                      {(logPrefs.showTimestamps || log.client_ip) && (
                        <div className="text-xs text-muted-foreground mt-1">
                          {logPrefs.showTimestamps && formatTimestamp(log.timestamp)}
                          {log.client_ip && (
                            <span className={logPrefs.showTimestamps ? 'ml-2' : ''}>
                              {logPrefs.showTimestamps ? '• ' : ''}
                              {log.client_ip}
                            </span>
                          )}
                        </div>
                      )}
                      {log.user_agent && !logPrefs.compactView && (
                        <div className="text-xs text-muted-foreground mt-1 truncate">
                          {log.user_agent}
                        </div>
                      )}
                    </div>
                  </div>

                  <div className="flex items-center gap-4">
                    {/* Response Time */}
                    <div className="text-right">
                      <div className="text-sm font-medium text-foreground">
                        {log.response_time_ms}ms
                      </div>
                      <div className="text-xs text-muted-foreground">
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

                    {/* View Trace Button */}
                    {log.id && (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setSelectedTraceRequestId(log.id!)}
                        className="flex items-center gap-2"
                        title="View response generation trace"
                      >
                        <Eye className="h-4 w-4" />
                        View Trace
                      </Button>
                    )}
                  </div>
                </div>
              ))}
              </div>

              {/* Load More Button */}
              {hasMoreToShow && (
                <div className="flex justify-center pt-4 border-t border-border">
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

      {/* Response Trace Modal */}
      {selectedTraceRequestId && (
        <ResponseTraceModal
          requestId={selectedTraceRequestId}
          open={!!selectedTraceRequestId}
          onOpenChange={(open) => {
            if (!open) {
              setSelectedTraceRequestId(null);
            }
          }}
        />
      )}
    </div>
  );
}
