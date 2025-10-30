/**
 * Overview metric cards for the analytics dashboard
 */

import React from 'react';
import { Card } from '../ui/Card';
import { TrendingUp, TrendingDown, Activity, AlertTriangle, Zap, Database } from 'lucide-react';
import type { OverviewMetrics } from '@/hooks/useAnalyticsV2';
import type { MetricsUpdate } from '@/hooks/useAnalyticsStream';

interface OverviewCardsProps {
  data: OverviewMetrics | MetricsUpdate | null;
  isLoading?: boolean;
}

export const OverviewCards: React.FC<OverviewCardsProps> = ({ data, isLoading }) => {
  if (isLoading || !data) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4">
        {[1, 2, 3, 4, 5, 6].map((i) => (
          <Card key={i} className="p-6 animate-pulse">
            <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-24 mb-2"></div>
            <div className="h-8 bg-gray-200 dark:bg-gray-700 rounded w-16"></div>
          </Card>
        ))}
      </div>
    );
  }

  const cards = [
    {
      title: 'Total Requests',
      value: data.total_requests.toLocaleString(),
      unit: '',
      icon: Activity,
      color: 'text-blue-600 dark:text-blue-400',
      bgColor: 'bg-blue-50 dark:bg-blue-900/20',
    },
    {
      title: 'Requests/sec',
      value: data.requests_per_second.toFixed(1),
      unit: '/s',
      icon: Zap,
      color: 'text-green-600 dark:text-green-400',
      bgColor: 'bg-green-50 dark:bg-green-900/20',
    },
    {
      title: 'Error Rate',
      value: data.error_rate.toFixed(2),
      unit: '%',
      icon: AlertTriangle,
      color:
        data.error_rate < 1
          ? 'text-green-600 dark:text-green-400'
          : data.error_rate < 5
          ? 'text-yellow-600 dark:text-yellow-400'
          : 'text-red-600 dark:text-red-400',
      bgColor:
        data.error_rate < 1
          ? 'bg-green-50 dark:bg-green-900/20'
          : data.error_rate < 5
          ? 'bg-yellow-50 dark:bg-yellow-900/20'
          : 'bg-red-50 dark:bg-red-900/20',
      badge: data.total_errors > 0 ? data.total_errors.toLocaleString() : undefined,
    },
    {
      title: 'Avg Latency',
      value: Math.round(data.avg_latency_ms),
      unit: 'ms',
      icon: TrendingUp,
      color:
        data.avg_latency_ms < 50
          ? 'text-green-600 dark:text-green-400'
          : data.avg_latency_ms < 200
          ? 'text-yellow-600 dark:text-yellow-400'
          : 'text-red-600 dark:text-red-400',
      bgColor:
        data.avg_latency_ms < 50
          ? 'bg-green-50 dark:bg-green-900/20'
          : data.avg_latency_ms < 200
          ? 'bg-yellow-50 dark:bg-yellow-900/20'
          : 'bg-red-50 dark:bg-red-900/20',
    },
    {
      title: 'P95 Latency',
      value: Math.round(data.p95_latency_ms),
      unit: 'ms',
      icon: TrendingUp,
      color:
        data.p95_latency_ms < 100
          ? 'text-green-600 dark:text-green-400'
          : data.p95_latency_ms < 500
          ? 'text-yellow-600 dark:text-yellow-400'
          : 'text-red-600 dark:text-red-400',
      bgColor:
        data.p95_latency_ms < 100
          ? 'bg-green-50 dark:bg-green-900/20'
          : data.p95_latency_ms < 500
          ? 'bg-yellow-50 dark:bg-yellow-900/20'
          : 'bg-red-50 dark:bg-red-900/20',
    },
    {
      title: 'Active Connections',
      value: Math.round(data.active_connections),
      unit: '',
      icon: Database,
      color: 'text-purple-600 dark:text-purple-400',
      bgColor: 'bg-purple-50 dark:bg-purple-900/20',
    },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4">
      {cards.map((card) => {
        const Icon = card.icon;
        return (
          <Card key={card.title} className="p-6 hover:shadow-lg transition-shadow">
            <div className="flex items-center justify-between mb-3">
              <div className="text-sm font-medium text-gray-600 dark:text-gray-400">
                {card.title}
              </div>
              <div className={`p-2 rounded-lg ${card.bgColor}`}>
                <Icon className={`h-4 w-4 ${card.color}`} />
              </div>
            </div>
            <div className="flex items-baseline justify-between">
              <div>
                <span className={`text-2xl font-bold ${card.color}`}>
                  {card.value}
                </span>
                {card.unit && (
                  <span className="text-sm text-gray-500 dark:text-gray-400 ml-1">
                    {card.unit}
                  </span>
                )}
              </div>
              {card.badge && (
                <span className="text-xs px-2 py-1 rounded-full bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400">
                  {card.badge} errors
                </span>
              )}
            </div>
          </Card>
        );
      })}
    </div>
  );
};
