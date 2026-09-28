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
import { Doughnut } from 'react-chartjs-2';
import { getChartPalette } from '../../utils/chartTheme';
import { computePillarScores, type PillarUsageMetrics } from '@/hooks/usePillarAnalytics';

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
  if (isLoading) {
    return (
      <div className="h-64 flex items-center justify-center">
        <div className="text-muted-foreground">Loading chart data...</div>
      </div>
    );
  }

  // No response (query idle or failed) or a response with no recorded usage:
  // say so rather than implying data is still on its way.
  const pillarScores = data ? computePillarScores(data) : null;
  if (!pillarScores || Object.values(pillarScores).every((score) => score === 0)) {
    return (
      <div className="h-64 flex items-center justify-center" data-testid="pillar-usage-chart-empty">
        <div className="text-center text-muted-foreground">
          <p>No pillar usage recorded for this time range.</p>
          <p className="text-sm mt-1">Usage appears here as mocks are served and features are used.</p>
        </div>
      </div>
    );
  }

  const palette = getChartPalette();

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
