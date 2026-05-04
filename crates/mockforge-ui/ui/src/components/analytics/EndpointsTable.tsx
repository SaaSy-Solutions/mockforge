import React, { useState } from 'react';
import { Card } from '../ui/Card';
import { Download } from 'lucide-react';
import type { EndpointMetrics } from '@/stores/useAnalyticsStore';
import { exportEndpointsToCSV } from '@/utils/exportData';

interface EndpointsTableProps {
  data: EndpointMetrics[];
  isLoading?: boolean;
}

type SortKey = 'request_rate' | 'avg_latency_ms' | 'p95_latency_ms' | 'error_rate_percent';
type SortOrder = 'asc' | 'desc';

export const EndpointsTable: React.FC<EndpointsTableProps> = ({ data, isLoading }) => {
  const [sortKey, setSortKey] = useState<SortKey>('request_rate');
  const [sortOrder, setSortOrder] = useState<SortOrder>('desc');

  const sortedData = React.useMemo(() => {
    if (!data) return [];

    return [...data].sort((a, b) => {
      const aValue = a[sortKey];
      const bValue = b[sortKey];

      if (sortOrder === 'asc') {
        return aValue > bValue ? 1 : -1;
      }
      return aValue < bValue ? 1 : -1;
    });
  }, [data, sortKey, sortOrder]);

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortKey(key);
      setSortOrder('desc');
    }
  };

  const SortIcon = ({ column }: { column: SortKey }) => {
    if (sortKey !== column) return <span className="text-muted-foreground">↕</span>;
    return sortOrder === 'asc' ? <span>↑</span> : <span>↓</span>;
  };

  if (isLoading) {
    return (
      <Card className="p-6">
        <h3 className="text-lg font-semibold mb-4">Top Endpoints</h3>
        <div className="animate-pulse space-y-3">
          {[1, 2, 3, 4, 5].map((i) => (
            <div key={i} className="h-12 bg-muted rounded"></div>
          ))}
        </div>
      </Card>
    );
  }

  if (!data || data.length === 0) {
    return (
      <Card className="p-6">
        <h3 className="text-lg font-semibold mb-4">Top Endpoints</h3>
        <div className="text-center py-8 text-muted-foreground">No endpoint data available</div>
      </Card>
    );
  }

  return (
    <Card className="p-6">
      <div className="flex justify-between items-center mb-4">
        <h3 className="text-lg font-semibold">Top Endpoints</h3>
        <button
          onClick={() => exportEndpointsToCSV(data)}
          className="px-3 py-1.5 text-sm bg-muted hover:bg-gray-200 dark:hover:bg-gray-700 rounded-lg flex items-center gap-2 transition-colors"
        >
          <Download className="w-4 h-4" />
          Export CSV
        </button>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border">
              <th className="text-left py-3 px-2 font-medium">Path</th>
              <th className="text-left py-3 px-2 font-medium">Method</th>
              <th
                className="text-right py-3 px-2 font-medium cursor-pointer hover:bg-accent hover:text-accent-foreground"
                onClick={() => handleSort('request_rate')}
              >
                Req/s <SortIcon column="request_rate" />
              </th>
              <th
                className="text-right py-3 px-2 font-medium cursor-pointer hover:bg-accent hover:text-accent-foreground"
                onClick={() => handleSort('avg_latency_ms')}
              >
                Avg Latency <SortIcon column="avg_latency_ms" />
              </th>
              <th
                className="text-right py-3 px-2 font-medium cursor-pointer hover:bg-accent hover:text-accent-foreground"
                onClick={() => handleSort('p95_latency_ms')}
              >
                P95 <SortIcon column="p95_latency_ms" />
              </th>
              <th
                className="text-right py-3 px-2 font-medium cursor-pointer hover:bg-accent hover:text-accent-foreground"
                onClick={() => handleSort('error_rate_percent')}
              >
                Error % <SortIcon column="error_rate_percent" />
              </th>
            </tr>
          </thead>
          <tbody>
            {sortedData.map((endpoint, index) => (
              <tr
                key={`${endpoint.path}-${endpoint.method}-${index}`}
                className="border-b border-border hover:bg-accent hover:text-accent-foreground/50"
              >
                <td className="py-3 px-2 font-mono text-xs">{endpoint.path}</td>
                <td className="py-3 px-2">
                  <span
                    className={`px-2 py-1 rounded text-xs font-medium ${
                      endpoint.method === 'GET'
                        ? 'bg-info-100 text-info-700 dark:bg-info-900/30 dark:text-info-400'
                        : endpoint.method === 'POST'
                        ? 'bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-400'
                        : endpoint.method === 'PUT'
                        ? 'bg-warning-100 text-warning-700 dark:bg-warning-900/30 dark:text-warning-400'
                        : endpoint.method === 'DELETE'
                        ? 'bg-danger-100 text-danger-700 dark:bg-danger-900/30 dark:text-danger-400'
                        : 'bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400'
                    }`}
                  >
                    {endpoint.method}
                  </span>
                </td>
                <td className="py-3 px-2 text-right">{endpoint.request_rate.toFixed(1)}</td>
                <td className="py-3 px-2 text-right">{Math.round(endpoint.avg_latency_ms)}ms</td>
                <td
                  className={`py-3 px-2 text-right ${
                    endpoint.p95_latency_ms < 100
                      ? 'text-success-600 dark:text-success-400'
                      : endpoint.p95_latency_ms < 500
                      ? 'text-warning-600 dark:text-warning-400'
                      : 'text-danger-600 dark:text-danger-400'
                  }`}
                >
                  {Math.round(endpoint.p95_latency_ms)}ms
                </td>
                <td
                  className={`py-3 px-2 text-right ${
                    endpoint.error_rate_percent < 1
                      ? 'text-success-600 dark:text-success-400'
                      : endpoint.error_rate_percent < 5
                      ? 'text-warning-600 dark:text-warning-400'
                      : 'text-danger-600 dark:text-danger-400'
                  }`}
                >
                  {endpoint.error_rate_percent.toFixed(2)}%
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  );
};
