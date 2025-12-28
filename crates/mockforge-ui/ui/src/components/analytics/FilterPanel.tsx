/**
 * Filter panel for analytics dashboard
 */

import React, { useState } from 'react';
import { Card } from '../ui/Card';
import { Filter, X } from 'lucide-react';
import type { AnalyticsFilter } from '@/hooks/useAnalyticsV2';

interface FilterPanelProps {
  filter: AnalyticsFilter;
  onChange: (filter: AnalyticsFilter) => void;
}

export const FilterPanel: React.FC<FilterPanelProps> = ({ filter, onChange }) => {
  const [isExpanded, setIsExpanded] = useState(false);

  const timeRanges = [
    { label: '5 minutes', value: 300 },
    { label: '15 minutes', value: 900 },
    { label: '1 hour', value: 3600 },
    { label: '6 hours', value: 21600 },
    { label: '24 hours', value: 86400 },
    { label: '7 days', value: 604800 },
  ];

  const granularities: Array<'minute' | 'hour' | 'day'> = ['minute', 'hour', 'day'];

  const handleReset = () => {
    onChange({
      duration: 3600,
      granularity: 'minute',
    });
  };

  const hasActiveFilters =
    filter.protocol ||
    filter.endpoint ||
    filter.method ||
    filter.workspace_id ||
    filter.environment;

  return (
    <Card className="p-4">
      <div className="flex items-center justify-between">
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex items-center gap-2 text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white"
        >
          <Filter className="h-4 w-4" />
          <span className="font-medium">Filters</span>
          {hasActiveFilters && (
            <span className="px-2 py-0.5 text-xs rounded-full bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300">
              Active
            </span>
          )}
        </button>

        {hasActiveFilters && (
          <button
            onClick={handleReset}
            className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
          >
            <X className="h-3 w-3" />
            Reset
          </button>
        )}
      </div>

      {isExpanded && (
        <div className="mt-4 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {/* Time range */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Time Range
            </label>
            <select
              value={filter.duration}
              onChange={(e) =>
                onChange({ ...filter, duration: parseInt(e.target.value) })
              }
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
            >
              {timeRanges.map((range) => (
                <option key={range.value} value={range.value}>
                  {range.label}
                </option>
              ))}
            </select>
          </div>

          {/* Granularity */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Granularity
            </label>
            <select
              value={filter.granularity}
              onChange={(e) =>
                onChange({
                  ...filter,
                  granularity: e.target.value as 'minute' | 'hour' | 'day',
                })
              }
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
            >
              {granularities.map((g) => (
                <option key={g} value={g}>
                  {g.charAt(0).toUpperCase() + g.slice(1)}
                </option>
              ))}
            </select>
          </div>

          {/* Protocol filter */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Protocol
            </label>
            <select
              value={filter.protocol || ''}
              onChange={(e) =>
                onChange({
                  ...filter,
                  protocol: e.target.value || undefined,
                })
              }
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
            >
              <option value="">All Protocols</option>
              <option value="HTTP">HTTP</option>
              <option value="gRPC">gRPC</option>
              <option value="WebSocket">WebSocket</option>
              <option value="GraphQL">GraphQL</option>
              <option value="MQTT">MQTT</option>
              <option value="Kafka">Kafka</option>
              <option value="AMQP">AMQP</option>
              <option value="SMTP">SMTP</option>
              <option value="FTP">FTP</option>
              <option value="TCP">TCP</option>
            </select>
          </div>

          {/* Endpoint filter */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Endpoint
            </label>
            <input
              type="text"
              value={filter.endpoint || ''}
              onChange={(e) =>
                onChange({
                  ...filter,
                  endpoint: e.target.value || undefined,
                })
              }
              placeholder="/api/users"
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white placeholder-gray-400"
            />
          </div>

          {/* Method filter */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Method
            </label>
            <select
              value={filter.method || ''}
              onChange={(e) =>
                onChange({
                  ...filter,
                  method: e.target.value || undefined,
                })
              }
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
            >
              <option value="">All Methods</option>
              <option value="GET">GET</option>
              <option value="POST">POST</option>
              <option value="PUT">PUT</option>
              <option value="PATCH">PATCH</option>
              <option value="DELETE">DELETE</option>
            </select>
          </div>

          {/* Environment filter */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Environment
            </label>
            <select
              value={filter.environment || ''}
              onChange={(e) =>
                onChange({
                  ...filter,
                  environment: e.target.value || undefined,
                })
              }
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
            >
              <option value="">All Environments</option>
              <option value="dev">Development</option>
              <option value="staging">Staging</option>
              <option value="prod">Production</option>
            </select>
          </div>
        </div>
      )}
    </Card>
  );
};
