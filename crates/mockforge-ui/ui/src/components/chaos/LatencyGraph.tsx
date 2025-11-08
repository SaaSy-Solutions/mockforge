/**
 * Real-time latency graph component
 * 
 * Displays request latency over time using a line chart.
 * Updates in real-time via polling (every 500ms).
 */

import React, { useMemo } from 'react';
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
import { useChaosLatencyMetrics, useChaosLatencyStats } from '../../hooks/useApi';
import { ModernCard } from '../ui/DesignSystem';
import { Spinner } from '../ui/LoadingStates';

// Register Chart.js components
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

interface LatencyGraphProps {
  /** Maximum number of data points to display (default: 100) */
  maxDataPoints?: number;
  /** Height of the chart in pixels (default: 300) */
  height?: number;
  /** Whether to show statistics overlay */
  showStats?: boolean;
}

export function LatencyGraph({
  maxDataPoints = 100,
  height = 300,
  showStats = true,
}: LatencyGraphProps) {
  const { data: metricsData, isLoading: metricsLoading, isError: metricsError } = useChaosLatencyMetrics();
  const { data: statsData, isLoading: statsLoading } = useChaosLatencyStats();

  // Transform data for Chart.js format
  const chartData = useMemo(() => {
    if (!metricsData?.samples || metricsData.samples.length === 0) {
      return {
        labels: [],
        datasets: [],
      };
    }

    // Take the most recent N samples
    const samples = metricsData.samples.slice(-maxDataPoints);

    // Transform to Chart.js format
    const labels = samples.map((sample) => {
      // Convert timestamp (ms) to relative time (seconds ago)
      const now = Date.now();
      const timeAgo = Math.max(0, (now - sample.timestamp) / 1000);
      
      if (timeAgo < 60) {
        return `${timeAgo.toFixed(0)}s`;
      }
      const minutes = Math.floor(timeAgo / 60);
      const remainingSeconds = Math.floor(timeAgo % 60);
      return `${minutes}m ${remainingSeconds}s`;
    });

    const latencyValues = samples.map((sample) => sample.latency_ms);

    return {
      labels,
      datasets: [
        {
          label: 'Request Latency',
          data: latencyValues,
          borderColor: 'rgb(59, 130, 246)',
          backgroundColor: 'rgba(59, 130, 246, 0.1)',
          borderWidth: 2,
          fill: true,
          tension: 0.4,
          pointRadius: 0,
          pointHoverRadius: 4,
        },
      ],
    };
  }, [metricsData, maxDataPoints]);

  const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        display: true,
        position: 'top' as const,
      },
      tooltip: {
        mode: 'index' as const,
        intersect: false,
        callbacks: {
          label: (context: any) => {
            return `Latency: ${context.parsed.y.toFixed(2)}ms`;
          },
        },
      },
    },
    scales: {
      x: {
        title: {
          display: true,
          text: 'Time Ago',
        },
        reverse: true, // Show most recent on right
      },
      y: {
        title: {
          display: true,
          text: 'Latency (ms)',
        },
        beginAtZero: true,
      },
    },
    animation: {
      duration: 300,
    },
  };

  if (metricsLoading && chartData.labels.length === 0) {
    return (
      <ModernCard>
        <div className="flex items-center justify-center" style={{ height: `${height}px` }}>
          <div className="text-center space-y-4">
            <Spinner size="lg" />
            <p className="text-gray-600 dark:text-gray-400">Loading latency data...</p>
          </div>
        </div>
      </ModernCard>
    );
  }

  if (metricsError) {
    return (
      <ModernCard>
        <div className="flex items-center justify-center" style={{ height: `${height}px` }}>
          <div className="text-center">
            <p className="text-red-600 dark:text-red-400">Failed to load latency data</p>
          </div>
        </div>
      </ModernCard>
    );
  }

  if (chartData.labels.length === 0) {
    return (
      <ModernCard>
        <div className="flex items-center justify-center" style={{ height: `${height}px` }}>
          <div className="text-center space-y-2">
            <p className="text-gray-600 dark:text-gray-400">No latency data available</p>
            <p className="text-sm text-gray-500 dark:text-gray-500">
              Enable latency injection to see real-time metrics
            </p>
          </div>
        </div>
      </ModernCard>
    );
  }

  return (
    <ModernCard>
      <div className="space-y-4">
        {/* Statistics overlay */}
        {showStats && statsData && !statsLoading && (
          <div className="flex items-center gap-6 text-sm">
            <div className="flex items-center gap-2">
              <span className="text-gray-500 dark:text-gray-400">Min:</span>
              <span className="font-semibold text-gray-900 dark:text-gray-100">
                {statsData.min_ms}ms
              </span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-gray-500 dark:text-gray-400">Avg:</span>
              <span className="font-semibold text-gray-900 dark:text-gray-100">
                {statsData.avg_ms.toFixed(1)}ms
              </span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-gray-500 dark:text-gray-400">Max:</span>
              <span className="font-semibold text-gray-900 dark:text-gray-100">
                {statsData.max_ms}ms
              </span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-gray-500 dark:text-gray-400">P95:</span>
              <span className="font-semibold text-gray-900 dark:text-gray-100">
                {statsData.p95_ms}ms
              </span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-gray-500 dark:text-gray-400">P99:</span>
              <span className="font-semibold text-gray-900 dark:text-gray-100">
                {statsData.p99_ms}ms
              </span>
            </div>
            <div className="flex items-center gap-2 ml-auto">
              <span className="text-gray-500 dark:text-gray-400">Samples:</span>
              <span className="font-semibold text-gray-900 dark:text-gray-100">
                {statsData.count}
              </span>
            </div>
          </div>
        )}

        {/* Chart */}
        <div style={{ height: `${height}px` }}>
          <Line data={chartData} options={chartOptions} />
        </div>
      </div>
    </ModernCard>
  );
}

