/**
 * Latency trend chart showing percentiles over time
 */

import React from 'react';
import { Line } from 'react-chartjs-2';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
} from 'chart.js';
import { Card } from '../ui/Card';
import { useLatencyTrends, type AnalyticsFilter } from '@/hooks/useAnalyticsV2';
import { TrendingUp } from 'lucide-react';
import { getChartPalette } from '../../utils/chartTheme';

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler
);

interface LatencyTrendChartProps {
  filter?: AnalyticsFilter;
}

export const LatencyTrendChart: React.FC<LatencyTrendChartProps> = ({ filter }) => {
  const { data, isLoading, error } = useLatencyTrends(filter);

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <TrendingUp className="h-5 w-5 text-muted-foreground" />
          <h3 className="text-lg font-semibold">Latency Trends</h3>
        </div>
        <div className="h-80 flex items-center justify-center">
          <div className="animate-pulse text-muted-foreground">Loading...</div>
        </div>
      </Card>
    );
  }

  if (error || !data?.trends || data.trends.length === 0) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <TrendingUp className="h-5 w-5 text-muted-foreground" />
          <h3 className="text-lg font-semibold">Latency Trends</h3>
        </div>
        <div className="h-80 flex items-center justify-center text-muted-foreground">
          {error ? 'Error loading data' : 'No data available'}
        </div>
      </Card>
    );
  }

  const palette = getChartPalette();
  const timestamps = data.trends.map((t) =>
    new Date(t.timestamp * 1000).toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
    })
  );

  const chartData = {
    labels: timestamps,
    datasets: [
      {
        label: 'P99',
        data: data.trends.map((t) => t.p99),
        borderColor: palette.danger,
        backgroundColor: palette.dangerAlpha(0.1),
        borderWidth: 2,
        pointRadius: 0,
        fill: false,
      },
      {
        label: 'P95',
        data: data.trends.map((t) => t.p95),
        borderColor: palette.warning,
        backgroundColor: palette.warningAlpha(0.1),
        borderWidth: 2,
        pointRadius: 0,
        fill: false,
      },
      {
        label: 'P50 (Median)',
        data: data.trends.map((t) => t.p50),
        borderColor: palette.success,
        backgroundColor: palette.successAlpha(0.1),
        borderWidth: 2,
        pointRadius: 0,
        fill: false,
      },
      {
        label: 'Average',
        data: data.trends.map((t) => t.avg),
        borderColor: palette.primary,
        backgroundColor: palette.primaryAlpha(0.1),
        borderWidth: 2,
        pointRadius: 0,
        fill: false,
        borderDash: [5, 5],
      },
    ],
  };

  const options = {
    responsive: true,
    maintainAspectRatio: false,
    interaction: {
      mode: 'index' as const,
      intersect: false,
    },
    plugins: {
      legend: {
        position: 'top' as const,
        labels: {
          usePointStyle: true,
          padding: 15,
        },
      },
      tooltip: {
        callbacks: {
          label: function (context: any) {
            let label = context.dataset.label || '';
            if (label) {
              label += ': ';
            }
            label += context.parsed.y.toFixed(1) + 'ms';
            return label;
          },
        },
      },
    },
    scales: {
      x: {
        grid: {
          display: false,
        },
      },
      y: {
        beginAtZero: true,
        title: {
          display: true,
          text: 'Latency (ms)',
        },
        ticks: {
          callback: function (value: any) {
            return value + 'ms';
          },
        },
      },
    },
  };

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <TrendingUp className="h-5 w-5 text-info-600 dark:text-info-400" />
          <h3 className="text-lg font-semibold">Latency Trends</h3>
        </div>
        <div className="text-sm text-muted-foreground">
          {data.trends.length} data points
        </div>
      </div>
      <div className="h-80">
        <Line data={chartData} options={options} />
      </div>
    </Card>
  );
};
