/**
 * Time range selector for pillar analytics
 */

import React from 'react';

interface TimeRangeSelectorProps {
  value: string;
  onChange: (value: string) => void;
}

const timeRanges = [
  { value: '1d', label: 'Last 24 Hours' },
  { value: '7d', label: 'Last 7 Days' },
  { value: '30d', label: 'Last 30 Days' },
  { value: '90d', label: 'Last 90 Days' },
  { value: 'all', label: 'All Time' },
];

export const TimeRangeSelector: React.FC<TimeRangeSelectorProps> = ({
  value,
  onChange,
}) => {
  return (
    <div className="flex items-center gap-2">
      <label className="text-sm font-medium text-foreground">
        Time Range:
      </label>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="px-3 py-2 border border-border rounded-md bg-card text-foreground text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
      >
        {timeRanges.map((range) => (
          <option key={range.value} value={range.value}>
            {range.label}
          </option>
        ))}
      </select>
    </div>
  );
};
