/**
 * Detailed view for Reality pillar metrics
 */

import React from 'react';
import { Card } from '../ui/Card';
import { Sparkles, Users, Database } from 'lucide-react';
import type { RealityPillarMetrics } from '@/hooks/usePillarAnalytics';

interface RealityPillarDetailsProps {
  data: RealityPillarMetrics;
  isLoading?: boolean;
  onSelect?: () => void;
  isSelected?: boolean;
}

export const RealityPillarDetails: React.FC<RealityPillarDetailsProps> = ({
  data,
  isLoading,
  onSelect,
  isSelected,
}) => {
  if (isLoading) {
    return (
      <Card className="p-6 animate-pulse">
        <div className="h-6 bg-gray-200 dark:bg-gray-700 rounded w-32 mb-4"></div>
        <div className="space-y-2">
          <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded"></div>
          <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded"></div>
        </div>
      </Card>
    );
  }

  return (
    <Card
      className={`p-6 cursor-pointer transition-all ${
        isSelected
          ? 'ring-2 ring-purple-500 shadow-lg'
          : 'hover:shadow-md'
      }`}
      onClick={onSelect}
    >
      <div className="flex items-center gap-3 mb-4">
        <Sparkles className="h-6 w-6 text-purple-600 dark:text-purple-400" />
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
          Reality Pillar
        </h3>
      </div>

      <div className="space-y-4">
        {/* Blended Reality */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Blended Reality Usage
            </span>
            <span className="text-lg font-bold text-purple-600 dark:text-purple-400">
              {data.blended_reality_percent.toFixed(1)}%
            </span>
          </div>
          <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
            <div
              className="bg-purple-600 h-2 rounded-full transition-all"
              style={{ width: `${data.blended_reality_percent}%` }}
            />
          </div>
        </div>

        {/* Personas vs Fixtures */}
        <div className="grid grid-cols-2 gap-4">
          <div className="flex items-center gap-2">
            <Users className="h-5 w-5 text-green-600 dark:text-green-400" />
            <div>
              <p className="text-sm font-medium text-gray-900 dark:text-white">
                {data.smart_personas_percent.toFixed(1)}%
              </p>
              <p className="text-xs text-gray-500 dark:text-gray-500">
                Smart Personas
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Database className="h-5 w-5 text-gray-600 dark:text-gray-400" />
            <div>
              <p className="text-sm font-medium text-gray-900 dark:text-white">
                {data.static_fixtures_percent.toFixed(1)}%
              </p>
              <p className="text-xs text-gray-500 dark:text-gray-500">
                Static Fixtures
              </p>
            </div>
          </div>
        </div>

        {/* Additional Metrics */}
        <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <p className="text-gray-500 dark:text-gray-500">Avg Reality Level</p>
              <p className="font-semibold text-gray-900 dark:text-white">
                {data.avg_reality_level.toFixed(1)}/5.0
              </p>
            </div>
            <div>
              <p className="text-gray-500 dark:text-gray-500">Total Scenarios</p>
              <p className="font-semibold text-gray-900 dark:text-white">
                {data.total_scenarios}
              </p>
            </div>
            <div>
              <p className="text-gray-500 dark:text-gray-500">Chaos Enabled</p>
              <p className="font-semibold text-gray-900 dark:text-white">
                {data.chaos_enabled_count}
              </p>
            </div>
          </div>
        </div>
      </div>
    </Card>
  );
};
