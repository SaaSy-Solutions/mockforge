import { logger } from '@/utils/logger';
import React, { useMemo } from 'react';
import { Pie, Bar } from 'react-chartjs-2';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  BarElement,
  ArcElement,
  Title,
  Tooltip,
  Legend,
} from 'chart.js';
import type { FailureMetrics } from '../../types';

// Register Chart.js components
ChartJS.register(
  CategoryScale,
  LinearScale,
  BarElement,
  ArcElement,
  Title,
  Tooltip,
  Legend
);

interface FailureCounterProps {
  metrics: FailureMetrics[];
  selectedService?: string;
  onServiceChange: (service: string) => void;
}

export function FailureCounter({ metrics, selectedService, onServiceChange }: FailureCounterProps) {
  const selectedMetric = selectedService ? metrics.find(m => m.service === selectedService) : metrics[0];

  const getStatusCodeColor = (code: number) => {
    if (code >= 200 && code < 300) return '#10b981'; // green
    if (code >= 300 && code < 400) return '#3b82f6'; // blue
    if (code >= 400 && code < 500) return '#f59e0b'; // yellow
    if (code >= 500) return '#ef4444'; // red
    return '#6b7280'; // gray
  };

  const formatErrorRate = (rate: number) => {
    return `${(rate * 100).toFixed(2)}%`;
  };

  // Pie chart data for success/failure
  const pieChartData = useMemo(() => {
    if (!selectedMetric) return { labels: [], datasets: [] };

    return {
      labels: ['Success', 'Failure'],
      datasets: [
        {
          data: [selectedMetric.success_count, selectedMetric.failure_count],
          backgroundColor: ['#10b981', '#ef4444'],
          borderColor: ['#10b981', '#ef4444'],
          borderWidth: 1,
        },
      ],
    };
  }, [selectedMetric]);

  const pieChartOptions = useMemo(() => ({
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        display: false,
      },
      tooltip: {
        callbacks: {
          label: (context: any) => {
            const value = context.parsed;
            const total = selectedMetric?.total_requests || 1;
            const percentage = ((value / total) * 100).toFixed(1);
            return `${context.label}: ${value} requests (${percentage}%)`;
          },
        },
      },
    },
  }), [selectedMetric]);

  // Bar chart data for status codes
  const barChartData = useMemo(() => {
    if (!selectedMetric) return { labels: [], datasets: [] };

    const statusCodeEntries = Object.entries(selectedMetric.status_codes || {});
    const codes = statusCodeEntries.map(([code]) => code);
    const counts = statusCodeEntries.map(([, count]) => count);
    const colors = statusCodeEntries.map(([code]) => getStatusCodeColor(parseInt(code)));

    return {
      labels: codes,
      datasets: [
        {
          label: 'Requests',
          data: counts,
          backgroundColor: colors,
          borderColor: colors,
          borderWidth: 1,
        },
      ],
    };
  }, [selectedMetric]);

  const barChartOptions = useMemo(() => ({
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        display: false,
      },
      tooltip: {
        callbacks: {
          label: (context: any) => `${context.parsed.y} requests`,
        },
      },
    },
    scales: {
      x: {
        grid: {
          display: true,
          color: 'rgba(0, 0, 0, 0.05)',
        },
        ticks: {
          font: {
            size: 12,
          },
        },
      },
      y: {
        beginAtZero: true,
        grid: {
          display: true,
          color: 'rgba(0, 0, 0, 0.05)',
        },
        ticks: {
          font: {
            size: 12,
          },
        },
      },
    },
  }), []);

  return (
    <div className="space-y-6">
      {/* Service Selector and Overview */}
      <div className="rounded-lg border bg-card p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold">Failure Analysis</h3>
          <select
            value={selectedService || ''}
            onChange={(e) => onServiceChange(e.target.value)}
            className="px-3 py-1 border border-input rounded text-sm bg-background"
          >
            <option value="">All Services</option>
            {metrics.map(metric => (
              <option key={metric.service} value={metric.service}>
                {metric.service}
              </option>
            ))}
          </select>
        </div>

        {selectedMetric && (
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-center">
            <div className="space-y-1">
              <div className="text-2xl font-bold">{(selectedMetric.total_requests || 0).toLocaleString()}</div>
              <div className="text-xs text-muted-foreground">Total Requests</div>
            </div>
            <div className="space-y-1">
              <div className="text-2xl font-bold text-green-600">{(selectedMetric.success_count || 0).toLocaleString()}</div>
              <div className="text-xs text-muted-foreground">Successful</div>
            </div>
            <div className="space-y-1">
              <div className="text-2xl font-bold text-red-600">{(selectedMetric.failure_count || 0).toLocaleString()}</div>
              <div className="text-xs text-muted-foreground">Failed</div>
            </div>
            <div className="space-y-1">
              <div className={`text-2xl font-bold ${(selectedMetric.error_rate || 0) < 0.05 ? 'text-green-600' : (selectedMetric.error_rate || 0) < 0.1 ? 'text-yellow-600' : 'text-red-600'}`}>
                {formatErrorRate(selectedMetric.error_rate || 0)}
              </div>
              <div className="text-xs text-muted-foreground">Error Rate</div>
            </div>
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Success/Failure Pie Chart */}
        <div className="rounded-lg border bg-card p-6">
          <h4 className="font-semibold mb-4">Success vs Failure</h4>

          {selectedMetric ? (
            <div className="h-64">
              <Pie data={pieChartData} options={pieChartOptions} />
            </div>
          ) : (
            <div className="flex items-center justify-center h-64 text-center">
              <div className="space-y-2">
                <div className="text-4xl">📈</div>
                <div className="text-muted-foreground">No failure data available</div>
              </div>
            </div>
          )}

          {selectedMetric && (
            <div className="flex justify-center space-x-6 mt-4">
              <div className="flex items-center space-x-2">
                <div className="w-3 h-3 bg-green-500 rounded"></div>
                <span className="text-sm">Success ({(((selectedMetric.success_count || 0) / (selectedMetric.total_requests || 1)) * 100).toFixed(1)}%)</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-3 h-3 bg-red-500 rounded"></div>
                <span className="text-sm">Failure ({(((selectedMetric.failure_count || 0) / (selectedMetric.total_requests || 1)) * 100).toFixed(1)}%)</span>
              </div>
            </div>
          )}
        </div>

        {/* Status Code Distribution */}
        <div className="rounded-lg border bg-card p-6">
          <h4 className="font-semibold mb-4">Status Code Distribution</h4>

          {selectedMetric && Object.keys(selectedMetric.status_codes || {}).length > 0 ? (
            <div className="h-64">
              <Bar data={barChartData} options={barChartOptions} />
            </div>
          ) : (
            <div className="flex items-center justify-center h-64 text-center">
              <div className="space-y-2">
                <div className="text-4xl">📊</div>
                <div className="text-muted-foreground">No status code data available</div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Error Rate Trend (placeholder for future implementation) */}
      <div className="rounded-lg border bg-card p-6">
        <h4 className="font-semibold mb-4">Error Rate Trend</h4>
        <div className="flex items-center justify-center h-32 text-center">
          <div className="space-y-2">
            <div className="text-4xl">📈</div>
            <div className="text-muted-foreground">
              Error rate trends will be available with time-series data
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}