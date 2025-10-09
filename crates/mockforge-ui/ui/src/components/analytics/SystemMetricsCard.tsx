import React from 'react';
import { Card } from '../ui/Card';
import type { SystemMetrics } from '@/stores/useAnalyticsStore';

interface SystemMetricsCardProps {
  data: SystemMetrics | null;
  isLoading?: boolean;
}

const formatUptime = (seconds: number): string => {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
};

export const SystemMetricsCard: React.FC<SystemMetricsCardProps> = ({ data, isLoading }) => {
  if (isLoading || !data) {
    return (
      <Card className="p-6 animate-pulse">
        <h3 className="text-lg font-semibold mb-4">System Health</h3>
        <div className="space-y-3">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="h-6 bg-gray-200 dark:bg-gray-700 rounded"></div>
          ))}
        </div>
      </Card>
    );
  }

  const metrics = [
    {
      label: 'Memory Usage',
      value: `${Math.round(data.memory_usage_mb)} MB`,
      color: 'text-blue-600 dark:text-blue-400',
    },
    {
      label: 'CPU Usage',
      value: `${data.cpu_usage_percent.toFixed(1)}%`,
      color:
        data.cpu_usage_percent < 50
          ? 'text-green-600 dark:text-green-400'
          : data.cpu_usage_percent < 80
          ? 'text-yellow-600 dark:text-yellow-400'
          : 'text-red-600 dark:text-red-400',
    },
    {
      label: 'Thread Count',
      value: Math.round(data.thread_count),
      color: 'text-purple-600 dark:text-purple-400',
    },
    {
      label: 'Uptime',
      value: formatUptime(data.uptime_seconds),
      color: 'text-gray-600 dark:text-gray-400',
    },
  ];

  return (
    <Card className="p-6">
      <h3 className="text-lg font-semibold mb-4">System Health</h3>
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
