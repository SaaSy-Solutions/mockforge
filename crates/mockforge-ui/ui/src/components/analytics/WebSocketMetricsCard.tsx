import React from 'react';
import { Card } from '../ui/Card';
import type { WebSocketMetrics } from '@/stores/useAnalyticsStore';

interface WebSocketMetricsCardProps {
  data: WebSocketMetrics | null;
  isLoading?: boolean;
}

export const WebSocketMetricsCard: React.FC<WebSocketMetricsCardProps> = ({ data, isLoading }) => {
  if (isLoading || !data) {
    return (
      <Card className="p-6 animate-pulse">
        <h3 className="text-lg font-semibold mb-4">WebSocket Metrics</h3>
        <div className="space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-6 bg-gray-200 dark:bg-gray-700 rounded"></div>
          ))}
        </div>
      </Card>
    );
  }

  const metrics = [
    {
      label: 'Active Connections',
      value: Math.round(data.active_connections),
      color: 'text-blue-600 dark:text-blue-400',
    },
    {
      label: 'Total Connections',
      value: Math.round(data.total_connections),
      color: 'text-gray-600 dark:text-gray-400',
    },
    {
      label: 'Messages Sent/s',
      value: data.message_rate_sent.toFixed(1),
      color: 'text-green-600 dark:text-green-400',
    },
    {
      label: 'Messages Received/s',
      value: data.message_rate_received.toFixed(1),
      color: 'text-purple-600 dark:text-purple-400',
    },
    {
      label: 'Error Rate',
      value: data.error_rate.toFixed(2),
      color:
        data.error_rate < 0.1
          ? 'text-green-600 dark:text-green-400'
          : 'text-red-600 dark:text-red-400',
    },
    {
      label: 'Avg Connection Duration',
      value: `${Math.round(data.avg_connection_duration_seconds)}s`,
      color: 'text-gray-600 dark:text-gray-400',
    },
  ];

  return (
    <Card className="p-6">
      <h3 className="text-lg font-semibold mb-4">WebSocket Metrics</h3>
      <div className="grid grid-cols-2 gap-4">
        {metrics.map((metric) => (
          <div key={metric.label} className="border-b border-gray-100 dark:border-gray-800 pb-2">
            <div className="text-sm text-gray-600 dark:text-gray-400">{metric.label}</div>
            <div className={`text-2xl font-semibold ${metric.color}`}>{metric.value}</div>
          </div>
        ))}
      </div>
    </Card>
  );
};
