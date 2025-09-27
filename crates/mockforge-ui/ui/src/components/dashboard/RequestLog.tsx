import React, { useMemo, useState, useEffect } from 'react';
import { cn } from '../../utils/cn';
import { FileText, Clock, Globe, Filter } from 'lucide-react';
import { Card } from '../ui/Card';
import { Input } from '../ui/input';
import { Button } from '../ui/button';
import { useLogs } from '../../hooks/useApi';
import { ResponsiveTable, type ResponsiveTableColumn } from '../ui/ResponsiveTable';
import { Badge } from '../ui/DesignSystem';
import { SkeletonTable } from '../ui/Skeleton';
import { useApiErrorHandling } from '../../hooks/useErrorHandling';
import { CompactErrorFallback, DataErrorFallback } from '../error/ErrorFallbacks';

type StatusFamily = 'all' | '2xx' | '4xx' | '5xx';

const methodClass = (m: string) =>
  m === 'GET'
    ? 'bg-info-50 text-info-600 bg-blue-100 text-blue-700 dark:bg-info-900/20 dark:text-info-400'
    : m === 'POST'
    ? 'bg-success-50 text-success-600 bg-green-100 text-green-700 dark:bg-success-900/20 dark:text-success-400'
    : m === 'PUT'
    ? 'bg-warning-50 text-warning-600 bg-yellow-100 text-yellow-700 dark:bg-warning-900/20 dark:text-warning-400'
    : m === 'DELETE'
    ? 'bg-danger-50 text-danger-600 bg-red-100 text-red-700 dark:bg-danger-900/20 dark:text-danger-400'
    : 'bg-neutral-100 text-neutral-700 dark:bg-neutral-800 dark:text-neutral-400';

export function RequestLog() {
  const [method, setMethod] = useState<string | null>(null);
  const [statusFamily, setStatusFamily] = useState<StatusFamily>('all');
  const [search, setSearch] = useState('');
  const [debounced, setDebounced] = useState('');

  const { handleApiError, retry, clearError, errorState, canRetry } = useApiErrorHandling();

  useEffect(() => {
    const t = setTimeout(() => setDebounced(search), 250);
    return () => clearTimeout(t);
  }, [search]);

  // Get logs with filters
  const { data: logsData, isLoading, error, refetch } = useLogs({
    method: method ?? undefined,
    path: debounced || undefined,
  });

  // Handle API errors
  useEffect(() => {
    if (error) {
      handleApiError(error, 'fetch_logs');
    } else {
      clearError();
    }
  }, [error, handleApiError, clearError]);

  const logs = useMemo(() => {
    if (!logsData) return [];

    if (statusFamily === 'all') return logsData;
    const start = statusFamily === '2xx' ? 200 : statusFamily === '4xx' ? 400 : 500;
    const end = start + 99;
    return logsData.filter((l: any) => l.status_code >= start && l.status_code <= end);
  }, [logsData, statusFamily]);

  // Define table columns
  const columns: ResponsiveTableColumn[] = [
    {
      key: 'method',
      label: 'Method',
      priority: 'high',
      render: (method: string) => (
        <span className={cn('text-xs font-semibold px-3 py-1.5 rounded-md uppercase tracking-wide', methodClass(method))}>
          {method}
        </span>
      ),
      width: '80px'
    },
    {
      key: 'path',
      label: 'Path',
      priority: 'high',
      mobileLabel: 'Endpoint',
      render: (path: string) => (
        <span className="font-mono text-sm text-primary truncate" title={path}>
          {path}
        </span>
      )
    },
    {
      key: 'status_code',
      label: 'Status',
      priority: 'high',
      render: (statusCode: number) => {
        return (
          <Badge variant={statusCode >= 200 && statusCode < 300 ? 'success' : 
                         statusCode >= 400 && statusCode < 500 ? 'warning' : 'error'}>
            {statusCode}
          </Badge>
        );
      },
      width: '80px'
    },
    {
      key: 'response_time_ms',
      label: 'Response Time',
      priority: 'medium',
      mobileLabel: 'Duration',
      render: (time: number) => (
        <span className="font-mono text-sm">
          {time}ms
        </span>
      ),
      width: '100px'
    },
    {
      key: 'timestamp',
      label: 'Time',
      priority: 'medium',
      render: (timestamp: string) => (
        <div className="flex items-center gap-1">
          <Clock className="h-3 w-3 text-tertiary" />
          <span className="font-mono text-sm">{fmtTime(timestamp)}</span>
        </div>
      ),
      width: '100px'
    },
    {
      key: 'client_ip',
      label: 'Client IP',
      priority: 'low',
      hideOnMobile: true,
      render: (ip: string) => (
        <div className="flex items-center gap-1">
          <Globe className="h-3 w-3 text-tertiary" />
          <span className="font-mono text-sm">{ip}</span>
        </div>
      ),
      width: '120px'
    }
  ];

  const fmtTime = (iso: string) => new Date(iso).toLocaleTimeString([], { hour12: false });

  const SegBtn = ({ active, children, onClick }: { active: boolean; children: React.ReactNode; onClick: () => void }) => (
    <Button
      variant="ghost"
      size="sm"
      className={cn(
        'px-3 h-7 text-xs font-medium transition-all duration-150',
        active
          ? 'bg-brand text-white shadow-sm hover:bg-brand-600'
          : 'text-text-secondary hover:text-text-primary hover:bg-bg-tertiary'
      )}
      onClick={onClick}
    >
      {children}
    </Button>
  );

  const reset = () => {
    setMethod(null);
    setStatusFamily('all');
    setSearch('');
  };

  if (isLoading) {
    return (
      <Card title="Recent Requests" icon={<FileText className="h-5 w-5" />}>
        <div className="space-y-6">
          {/* Filter skeleton */}
          <div className="flex flex-col xl:flex-row xl:items-center xl:justify-between gap-4">
            <div className="flex flex-col sm:flex-row sm:items-center gap-4">
              <SkeletonTable rows={1} cols={3} className="h-8" />
            </div>
            <SkeletonTable rows={1} cols={1} className="h-10 w-96" />
          </div>
          
          {/* Table skeleton */}
          <SkeletonTable rows={5} cols={5} />
        </div>
      </Card>
    );
  }

  if (errorState.error) {
    return (
      <Card title="Recent Requests" icon={<FileText className="h-5 w-5" />}>
        <DataErrorFallback 
          retry={canRetry ? () => retry(async () => { await refetch(); }) : undefined}
          resetError={clearError}
        />
      </Card>
    );
  }

  return (
    <Card title="Recent Requests" icon={<FileText className="h-5 w-5" />}>
      {/* Filter Controls - Optimized for Full Width */}
      <div className="mb-6">
        <div className="flex flex-col xl:flex-row xl:items-center xl:justify-between gap-4">
          {/* Left Side - Filters */}
          <div className="flex flex-col sm:flex-row sm:items-center gap-4">
            <div className="flex items-center gap-2">
              <Filter className="h-4 w-4 text-text-tertiary" />
              <span className="text-sm font-medium text-text-secondary">Filters:</span>
            </div>

            {/* Status Code Filters */}
            <div className="flex items-center gap-2">
              <span className="text-xs text-text-tertiary uppercase tracking-wide font-medium">Status</span>
              <div className="inline-flex rounded-lg border border-border bg-bg-secondary p-1">
                {(['all','2xx','4xx','5xx'] as StatusFamily[]).map(sf => (
                  <SegBtn key={sf} active={statusFamily === sf} onClick={() => setStatusFamily(sf)}>
                    {sf.toUpperCase()}
                  </SegBtn>
                ))}
              </div>
            </div>

            {/* Method Filters */}
            <div className="flex items-center gap-2">
              <span className="text-xs text-text-tertiary uppercase tracking-wide font-medium">Method</span>
              <div className="inline-flex rounded-lg border border-border bg-bg-secondary p-1">
                {(['ALL','GET','POST','PUT','DELETE','PATCH'] as const).map(m => (
                  <SegBtn key={m} active={(method ?? 'ALL') === m} onClick={() => setMethod(m === 'ALL' ? null : m)}>
                    {m}
                  </SegBtn>
                ))}
              </div>
            </div>
          </div>

          {/* Right Side - Search and Clear */}
          <div className="flex items-center gap-2 w-full xl:w-96">
            <Input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search path, method, errors…"
              className="flex-1"
            />
            <Button variant="outline" size="sm" onClick={reset} className="shrink-0">
              Clear
            </Button>
          </div>
        </div>
      </div>

      {/* Request List */}
      <div className="max-h-80 overflow-y-auto">
        <ResponsiveTable
          columns={columns}
          data={logs}
          stackOnMobile={true}
          sortable={true}
          emptyMessage="No requests found"
          className="animate-fade-in-up"
        />
      </div>
    </Card>
  );
}
