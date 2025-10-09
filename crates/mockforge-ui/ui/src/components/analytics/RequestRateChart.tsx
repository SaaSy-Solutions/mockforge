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
} from 'chart.js';
import type { ChartOptions } from 'chart.js';
import { Card } from '../ui/Card';
import type { RequestMetrics } from '@/stores/useAnalyticsStore';

// Register Chart.js components
ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend
);

interface RequestRateChartProps {
  data: RequestMetrics | null;
  isLoading?: boolean;
}

export const RequestRateChart: React.FC<RequestRateChartProps> = ({ data, isLoading }) => {
  const chartData = useMemo(() => {
    if (!data) {
      return {
        labels: [],
        datasets: [],
      };
    }

    // Convert timestamps to readable labels
    const labels = data.timestamps.map((ts) => {
      const date = new Date(ts * 1000);
      return date.toLocaleTimeString();
    });

    // Color palette for different protocols
    const colors = [
      { border: 'rgb(59, 130, 246)', bg: 'rgba(59, 130, 246, 0.1)' }, // blue
      { border: 'rgb(16, 185, 129)', bg: 'rgba(16, 185, 129, 0.1)' }, // green
      { border: 'rgb(249, 115, 22)', bg: 'rgba(249, 115, 22, 0.1)' }, // orange
      { border: 'rgb(168, 85, 247)', bg: 'rgba(168, 85, 247, 0.1)' }, // purple
      { border: 'rgb(236, 72, 153)', bg: 'rgba(236, 72, 153, 0.1)' }, // pink
    ];

    const datasets = data.series.map((series, index) => ({
      label: series.name,
      data: series.values,
      borderColor: colors[index % colors.length].border,
      backgroundColor: colors[index % colors.length].bg,
      borderWidth: 2,
      pointRadius: 2,
      pointHoverRadius: 4,
      tension: 0.4,
    }));

    return {
      labels,
      datasets,
    };
  }, [data]);

  const options: ChartOptions<'line'> = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        position: 'top' as const,
        labels: {
          usePointStyle: true,
          padding: 15,
        },
      },
      title: {
        display: false,
      },
      tooltip: {
        mode: 'index',
        intersect: false,
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
          text: 'Requests per second',
        },
      },
    },
    interaction: {
      mode: 'nearest',
      axis: 'x',
      intersect: false,
    },
  };

  if (isLoading) {
    return (
      <Card className="p-6">
        <h3 className="text-lg font-semibold mb-4">Request Rate by Protocol</h3>
        <div className="h-64 flex items-center justify-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        </div>
      </Card>
    );
  }

  if (!data || data.series.length === 0) {
    return (
      <Card className="p-6">
        <h3 className="text-lg font-semibold mb-4">Request Rate by Protocol</h3>
        <div className="h-64 flex items-center justify-center text-gray-500">
          No data available
        </div>
      </Card>
    );
  }

  return (
    <Card className="p-6">
      <h3 className="text-lg font-semibold mb-4">Request Rate by Protocol</h3>
      <div className="h-64">
        <Line data={chartData} options={options} />
      </div>
    </Card>
  );
};
