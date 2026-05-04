import React from 'react';
import { Card } from '../ui/Card';
import type { SummaryMetrics } from '@/stores/useAnalyticsStore';

interface SummaryCardsProps {
  data: SummaryMetrics | null;
  isLoading?: boolean;
}

export const SummaryCards: React.FC<SummaryCardsProps> = ({ data, isLoading }) => {
  if (isLoading || !data) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {[1, 2, 3, 4].map((i) => (
          <Card key={i} className="p-6 animate-pulse">
            <div className="h-4 bg-muted rounded w-24 mb-2"></div>
            <div className="h-8 bg-muted rounded w-16"></div>
          </Card>
        ))}
      </div>
    );
  }

  const cards = [
    {
      title: 'Request Rate',
      value: data.request_rate.toFixed(1),
      unit: 'req/s',
      color: 'text-info-600 dark:text-info-400',
    },
    {
      title: 'P95 Latency',
      value: Math.round(data.p95_latency_ms),
      unit: 'ms',
      color:
        data.p95_latency_ms < 100
          ? 'text-success-600 dark:text-success-400'
          : data.p95_latency_ms < 500
          ? 'text-warning-600 dark:text-warning-400'
          : 'text-danger-600 dark:text-danger-400',
    },
    {
      title: 'Error Rate',
      value: data.error_rate_percent.toFixed(2),
      unit: '%',
      color:
        data.error_rate_percent < 1
          ? 'text-success-600 dark:text-success-400'
          : data.error_rate_percent < 5
          ? 'text-warning-600 dark:text-warning-400'
          : 'text-danger-600 dark:text-danger-400',
    },
    {
      title: 'Active Connections',
      value: Math.round(data.active_connections),
      unit: '',
      color: 'text-purple-600 dark:text-purple-400',
    },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {cards.map((card) => (
        <Card key={card.title} className="p-6">
          <div className="text-sm font-medium text-muted-foreground mb-1">
            {card.title}
          </div>
          <div className={`text-3xl font-bold ${card.color}`}>
            {card.value}
            <span className="text-lg ml-1">{card.unit}</span>
          </div>
        </Card>
      ))}
    </div>
  );
};
