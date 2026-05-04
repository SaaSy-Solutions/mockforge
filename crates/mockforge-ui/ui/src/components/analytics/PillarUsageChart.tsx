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
import { getChartPalette } from '../../utils/chartTheme';
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
        <div className="text-muted-foreground">Loading chart data...</div>
      </div>
    );
  }

  const palette = getChartPalette();

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
          palette.infoAlpha(0.8),
          palette.successAlpha(0.8),
          palette.warningAlpha(0.8),
          palette.primaryAlpha(0.8),
          palette.dangerAlpha(0.8),
        ],
        borderColor: [
          palette.info,
          palette.success,
          palette.warning,
          palette.primary,
          palette.danger,
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
