/**
 * Traffic heatmap showing request patterns by hour and day of week
 */

import React from 'react';
import { Card } from '../ui/Card';
import { Calendar } from 'lucide-react';
import { useTrafficPatterns } from '@/hooks/useAnalyticsV2';

interface TrafficHeatmapProps {
  days?: number;
  workspace_id?: string;
}

export const TrafficHeatmap: React.FC<TrafficHeatmapProps> = ({ days = 7, workspace_id }) => {
  const { data, isLoading, error } = useTrafficPatterns(days, workspace_id);

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <Calendar className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Traffic Patterns</h3>
        </div>
        <div className="h-96 flex items-center justify-center">
          <div className="animate-pulse text-gray-400">Loading...</div>
        </div>
      </Card>
    );
  }

  if (error || !data?.patterns || data.patterns.length === 0) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <Calendar className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Traffic Patterns</h3>
        </div>
        <div className="h-96 flex items-center justify-center text-gray-400">
          {error ? 'Error loading data' : 'No data available'}
        </div>
      </Card>
    );
  }

  const dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  const hours = Array.from({ length: 24 }, (_, i) => i);

  // Create a map for quick lookup
  const patternMap = new Map<string, number>();
  data.patterns.forEach((p) => {
    const key = `${p.day_of_week}-${p.hour}`;
    patternMap.set(key, p.request_count);
  });

  // Find max value for color scaling
  const maxValue = Math.max(...data.patterns.map((p) => p.request_count), 1);

  const getColor = (value: number) => {
    if (value === 0) return 'bg-gray-100 dark:bg-gray-800';
    const intensity = value / maxValue;
    if (intensity < 0.2) return 'bg-blue-100 dark:bg-blue-900/30';
    if (intensity < 0.4) return 'bg-blue-200 dark:bg-blue-800/40';
    if (intensity < 0.6) return 'bg-blue-300 dark:bg-blue-700/50';
    if (intensity < 0.8) return 'bg-blue-400 dark:bg-blue-600/60';
    return 'bg-blue-500 dark:bg-blue-500/70';
  };

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Calendar className="h-5 w-5 text-blue-600 dark:text-blue-400" />
          <h3 className="text-lg font-semibold">Traffic Heatmap</h3>
        </div>
        <div className="text-sm text-gray-500 dark:text-gray-400">
          Requests by hour and day of week
        </div>
      </div>

      <div className="overflow-x-auto">
        <div className="inline-block min-w-full">
          {/* Hour labels */}
          <div className="flex mb-2">
            <div className="w-16" />
            {hours.map((hour) => (
              <div
                key={hour}
                className="flex-shrink-0 w-8 text-xs text-center text-gray-600 dark:text-gray-400"
              >
                {hour}
              </div>
            ))}
          </div>

          {/* Heatmap grid */}
          {dayNames.map((dayName, dayIndex) => (
            <div key={dayIndex} className="flex mb-1">
              {/* Day label */}
              <div className="w-16 text-sm text-gray-700 dark:text-gray-300 flex items-center">
                {dayName}
              </div>

              {/* Hour cells */}
              {hours.map((hour) => {
                const key = `${dayIndex}-${hour}`;
                const value = patternMap.get(key) || 0;
                const color = getColor(value);

                return (
                  <div
                    key={hour}
                    className={`
                      flex-shrink-0 w-8 h-8 mx-0.5 rounded ${color}
                      hover:ring-2 hover:ring-blue-500 cursor-pointer
                      transition-all
                    `}
                    title={`${dayName} ${hour}:00 - ${value.toLocaleString()} requests`}
                  />
                );
              })}
            </div>
          ))}

          {/* Legend */}
          <div className="flex items-center justify-center gap-2 mt-4 text-xs text-gray-600 dark:text-gray-400">
            <span>Less</span>
            <div className="flex gap-1">
              <div className="w-4 h-4 bg-gray-100 dark:bg-gray-800 rounded" />
              <div className="w-4 h-4 bg-blue-100 dark:bg-blue-900/30 rounded" />
              <div className="w-4 h-4 bg-blue-200 dark:bg-blue-800/40 rounded" />
              <div className="w-4 h-4 bg-blue-300 dark:bg-blue-700/50 rounded" />
              <div className="w-4 h-4 bg-blue-400 dark:bg-blue-600/60 rounded" />
              <div className="w-4 h-4 bg-blue-500 dark:bg-blue-500/70 rounded" />
            </div>
            <span>More</span>
          </div>
        </div>
      </div>
    </Card>
  );
};
