/**
 * Request count time-series chart
 */

import React from 'react';
import { Line } from 'react-chartjs-2';
import { Card } from '../ui/Card';
import { useRequestTimeSeries, type AnalyticsFilter } from '@/hooks/useAnalyticsV2';
import { Activity } from 'lucide-react';

interface RequestTimeSeriesChartProps {
  filter?: AnalyticsFilter;
}

export const RequestTimeSeriesChart: React.FC<RequestTimeSeriesChartProps> = ({ filter }) => {
  const { data, isLoading, error } = useRequestTimeSeries(filter);

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <Activity className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Request Rate</h3>
        </div>
        <div className="h-80 flex items-center justify-center">
          <div className="animate-pulse text-gray-400">Loading...</div>
        </div>
      </Card>
    );
  }

  if (error || !data?.series || data.series.length === 0) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <Activity className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Request Rate</h3>
        </div>
        <div className="h-80 flex items-center justify-center text-gray-400">
          {error ? 'Error loading data' : 'No data available'}
        </div>
      </Card>
    );
  }

  // Get unique timestamps from first series
  const timestamps =
    data.series[0]?.data.map((d) =>
      new Date(d.timestamp * 1000).toLocaleTimeString([], {
        hour: '2-digit',
        minute: '2-digit',
      })
    ) || [];

  const colors = [
    { border: 'rgb(59, 130, 246)', bg: 'rgba(59, 130, 246, 0.1)' }, // blue
    { border: 'rgb(34, 197, 94)', bg: 'rgba(34, 197, 94, 0.1)' }, // green
    { border: 'rgb(168, 85, 247)', bg: 'rgba(168, 85, 247, 0.1)' }, // purple
    { border: 'rgb(245, 158, 11)', bg: 'rgba(245, 158, 11, 0.1)' }, // orange
    { border: 'rgb(236, 72, 153)', bg: 'rgba(236, 72, 153, 0.1)' }, // pink
  ];

  const chartData = {
    labels: timestamps,
    datasets: data.series.map((series, index) => ({
      label: series.label,
      data: series.data.map((d) => d.value),
      borderColor: colors[index % colors.length].border,
      backgroundColor: colors[index % colors.length].bg,
      borderWidth: 2,
      pointRadius: 0,
      fill: true,
      tension: 0.4,
    })),
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
            label += context.parsed.y.toFixed(1) + ' req/s';
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
        stacked: false,
        title: {
          display: true,
          text: 'Requests per Second',
        },
        ticks: {
          callback: function (value: any) {
            return value + ' req/s';
          },
        },
      },
    },
  };

  const totalRequests = data.series.reduce(
    (sum, series) => sum + series.data.reduce((s, d) => s + d.value, 0),
    0
  );

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Activity className="h-5 w-5 text-blue-600 dark:text-blue-400" />
          <h3 className="text-lg font-semibold">Request Rate by Protocol</h3>
        </div>
        <div className="text-sm text-gray-500 dark:text-gray-400">
          Total: {totalRequests.toFixed(0)} requests
        </div>
      </div>
      <div className="h-80">
        <Line data={chartData} options={options} />
      </div>
    </Card>
  );
};
