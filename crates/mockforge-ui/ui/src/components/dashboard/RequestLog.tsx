import React, { useMemo, useState, useEffect } from 'react';
import { cn } from '../../utils/cn';
import { StatusBadge } from '../ui/StatusBadge';
import { FileText, Clock, Globe, Filter, Loader2 } from 'lucide-react';
import { Card } from '../ui/Card';
import { Input } from '../ui/input';
import { Button } from '../ui/button';
import { useLogs } from '../../hooks/useApi';

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

  useEffect(() => {
    const t = setTimeout(() => setDebounced(search), 250);
    return () => clearTimeout(t);
  }, [search]);

  // Get logs with filters
  const { data: logsData, isLoading, error } = useLogs({
    method: method ?? undefined,
    path: debounced || undefined,
  });

  const logs = useMemo(() => {
    if (!logsData) return [];

    if (statusFamily === 'all') return logsData;
    const start = statusFamily === '2xx' ? 200 : statusFamily === '4xx' ? 400 : 500;
    const end = start + 99;
    return logsData.filter((l: any) => l.status_code >= start && l.status_code <= end);
  }, [logsData, statusFamily]);

  const statusToBadge = (code: number): 'running' | 'warning' | 'error' | 'info' => {
    if (code >= 200 && code < 300) return 'running';
    if (code >= 400 && code < 500) return 'warning';
    if (code >= 500) return 'error';
    return 'info';
  };

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
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-text-secondary" />
          <span className="ml-2 text-text-secondary">Loading logs...</span>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card title="Recent Requests" icon={<FileText className="h-5 w-5" />}>
        <div className="py-8 text-center">
          <div className="text-red-500 mb-2">Failed to load logs</div>
          <div className="text-sm text-text-secondary">
            {error instanceof Error ? error.message : 'Unknown error'}
          </div>
        </div>
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
      <div className="max-h-80 overflow-y-auto custom-scrollbar">
        {logs.length === 0 ? (
          <div className="py-12 text-center">
            <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-bg-tertiary mb-4">
              <FileText className="h-8 w-8 text-text-tertiary" />
            </div>
            <div className="text-lg font-medium text-text-primary mb-2">No requests found</div>
            <div className="text-sm text-text-tertiary max-w-sm mx-auto">
              {method || search || statusFamily !== 'all'
                ? 'Try adjusting your filters to see more results'
                : 'Requests will appear here as they come in'
              }
            </div>
          </div>
        ) : (
          <div className="space-y-1">
            {logs.map((r: any) => (
              <div key={r.id} className="group p-4 rounded-lg hover:bg-bg-tertiary/50 transition-all duration-150 border border-transparent hover:border-border/50">
                <div className="flex items-center gap-6">
                  {/* Method Badge */}
                  <div className="flex-shrink-0">
                    <span className={cn('text-xs font-semibold px-3 py-1.5 rounded-md uppercase tracking-wide', methodClass(r.method))}>
                      {r.method}
                    </span>
                  </div>

                  {/* Request Path */}
                  <div className="flex-1 min-w-0">
                    <div className="font-mono text-sm text-text-primary truncate" title={r.path}>
                      {r.path}
                    </div>
                  </div>

                  {/* Time and IP */}
                  <div className="flex items-center gap-4 text-xs text-text-tertiary flex-shrink-0">
                    <div className="flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      <span className="font-mono">{fmtTime(r.timestamp)}</span>
                    </div>
                    <div className="flex items-center gap-1">
                      <Globe className="h-3 w-3" />
                      <span className="font-mono">{r.client_ip}</span>
                    </div>
                    <div className="font-mono font-medium">
                      {r.response_time_ms}ms
                    </div>
                  </div>

                  {/* Status Badge */}
                  <div className="flex-shrink-0">
                    <StatusBadge status={statusToBadge(r.status_code)} size="sm" />
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </Card>
  );
}
