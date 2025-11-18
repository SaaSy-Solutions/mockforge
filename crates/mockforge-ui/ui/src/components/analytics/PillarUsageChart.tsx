/**
 * Chart showing pillar usage distribution
 */

import React from 'react';
import {
  Chart as ChartJS,
  ArcElement,
  Tooltip,
  Legend,
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
} from 'chart.js';
import { Doughnut, Bar } from 'react-chartjs-2';
import type { PillarUsageMetrics } from '@/hooks/usePillarAnalytics';

ChartJS.register(
  ArcElement,
  Tooltip,
  Legend,
  CategoryScale,
  LinearScale,
  BarElement,
  Title
);

interface PillarUsageChartProps {
  data: PillarUsageMetrics | null | undefined;
  isLoading?: boolean;
}

export const PillarUsageChart: React.FC<PillarUsageChartProps> = ({
  data,
  isLoading,
}) => {
  if (isLoading || !data) {
    return (
      <div className="h-64 flex items-center justify-center">
        <div className="text-gray-500 dark:text-gray-400">Loading chart data...</div>
      </div>
    );
  }

  // Calculate pillar usage scores (normalized 0-100)
  const pillarScores = {
    reality: data.reality?.blended_reality_percent ?? 0,
    contracts: data.contracts?.validation_enforce_percent ?? 0,
    devx: data.devx ? (data.devx.sdk_installations > 0 ? 50 : 0) : 0,
    cloud: data.cloud ? (data.cloud.shared_scenarios_count > 0 ? 50 : 0) : 0,
    ai: data.ai ? (data.ai.ai_generated_mocks > 0 ? 50 : 0) : 0,
  };

  const chartData = {
    labels: ['Reality', 'Contracts', 'DevX', 'Cloud', 'AI'],
    datasets: [
      {
        label: 'Pillar Usage Score',
        data: [
          pillarScores.reality,
          pillarScores.contracts,
          pillarScores.devx,
          pillarScores.cloud,
          pillarScores.ai,
        ],
        backgroundColor: [
          'rgba(147, 51, 234, 0.8)', // purple
          'rgba(37, 99, 235, 0.8)',  // blue
          'rgba(34, 197, 94, 0.8)',  // green
          'rgba(249, 115, 22, 0.8)', // orange
          'rgba(236, 72, 153, 0.8)', // pink
        ],
        borderColor: [
          'rgba(147, 51, 234, 1)',
          'rgba(37, 99, 235, 1)',
          'rgba(34, 197, 94, 1)',
          'rgba(249, 115, 22, 1)',
          'rgba(236, 72, 153, 1)',
        ],
        borderWidth: 2,
      },
    ],
  };

  const options = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        position: 'right' as const,
      },
      tooltip: {
        callbacks: {
          label: (context: any) => {
            return `${context.label}: ${context.parsed.toFixed(1)}%`;
          },
        },
      },
    },
  };

  return (
    <div className="h-64">
      <Doughnut data={chartData} options={options} />
    </div>
  );
};
