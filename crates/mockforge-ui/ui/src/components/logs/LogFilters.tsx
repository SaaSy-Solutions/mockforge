import React from 'react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Switch } from '../ui/switch';
import type { LogFilter } from '../../types';

interface LogFiltersProps {
  filter: LogFilter;
  onFilterChange: (filter: Partial<LogFilter>) => void;
  onClearFilters: () => void;
  autoScroll: boolean;
  onAutoScrollChange: (enabled: boolean) => void;
  isPaused: boolean;
  onPauseChange: (paused: boolean) => void;
}

export function LogFilters({
  filter,
  onFilterChange,
  onClearFilters,
  autoScroll,
  onAutoScrollChange,
  isPaused,
  onPauseChange
}: LogFiltersProps) {
  const statusCodes = [200, 201, 204, 301, 302, 400, 401, 403, 404, 422, 500, 502, 503];
  const methods = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE'];
  const levels = ['debug', 'info', 'warn', 'error'] as const;

  return (
    <div className="space-y-4 p-4 bg-muted/30 border-b">
      {/* Controls Row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-2">
            <Switch
              checked={!isPaused}
              onCheckedChange={(checked) => onPauseChange(!checked)}
            />
            <span className="text-sm font-medium">
              {isPaused ? '⏸️ Paused' : '▶️ Live'}
            </span>
          </div>

          <div className="flex items-center space-x-2">
            <Switch
              checked={autoScroll}
              onCheckedChange={onAutoScrollChange}
              disabled={isPaused}
            />
            <span className="text-sm">Auto-scroll</span>
          </div>
        </div>

        <div className="flex items-center space-x-2">
          <Button variant="outline" size="sm" onClick={onClearFilters}>
            Clear Filters
          </Button>
          <Button variant="outline" size="sm">
            Export Logs
          </Button>
        </div>
      </div>

      {/* Filter Controls */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {/* Search */}
        <div className="space-y-2">
          <label className="text-sm font-medium">Search</label>
          <Input
            placeholder="Search logs..."
            value={filter.path_pattern || ''}
            onChange={(e) => onFilterChange({ path_pattern: e.target.value || undefined })}
          />
        </div>

        {/* Method Filter */}
        <div className="space-y-2">
          <label className="text-sm font-medium">Method</label>
          <select
            value={filter.method || ''}
            onChange={(e) => onFilterChange({ method: e.target.value || undefined })}
            className="w-full px-3 py-2 border border-input rounded-md bg-background text-foreground text-sm"
          >
            <option value="">All methods</option>
            {methods.map(method => (
              <option key={method} value={method}>{method}</option>
            ))}
          </select>
        </div>

        {/* Status Code Filter */}
        <div className="space-y-2">
          <label className="text-sm font-medium">Status Code</label>
          <select
            value={filter.status_code?.toString() || ''}
            onChange={(e) => onFilterChange({ status_code: e.target.value ? Number(e.target.value) : undefined })}
            className="w-full px-3 py-2 border border-input rounded-md bg-background text-foreground text-sm"
          >
            <option value="">All status codes</option>
            {statusCodes.map(code => (
              <option key={code} value={code}>{code}</option>
            ))}
          </select>
        </div>

        {/* Log Level Filter */}
        <div className="space-y-2">
          <label className="text-sm font-medium">Level</label>
          <select
            value={filter.level || ''}
            onChange={(e) => onFilterChange({ level: (e.target.value as typeof levels[number]) || undefined })}
            className="w-full px-3 py-2 border border-input rounded-md bg-background text-foreground text-sm"
          >
            <option value="">All levels</option>
            {levels.map(level => (
              <option key={level} value={level}>
                {level.charAt(0).toUpperCase() + level.slice(1)}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* Time Range */}
      <div className="flex items-center space-x-4">
        <label className="text-sm font-medium">Time range:</label>
        <div className="flex items-center space-x-2">
          {[1, 6, 24, 168].map(hours => (
            <button
              key={hours}
              onClick={() => onFilterChange({ hours_ago: hours })}
              className={`px-3 py-1 text-xs rounded ${
                filter.hours_ago === hours
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-secondary text-secondary-foreground hover:bg-secondary/80'
              }`}
            >
              {hours === 1 ? '1h' : hours === 6 ? '6h' : hours === 24 ? '24h' : '7d'}
            </button>
          ))}
        </div>

        <div className="flex items-center space-x-2">
          <label className="text-sm">Limit:</label>
          <select
            value={filter.limit || 100}
            onChange={(e) => onFilterChange({ limit: Number(e.target.value) })}
            className="px-2 py-1 border border-input rounded text-sm bg-background"
          >
            <option value={50}>50</option>
            <option value={100}>100</option>
            <option value={500}>500</option>
            <option value={1000}>1000</option>
          </select>
        </div>
      </div>

      {/* Active Filters Summary */}
      {(filter.method || filter.status_code || filter.path_pattern || filter.level) && (
        <div className="flex items-center space-x-2 text-sm">
          <span className="text-muted-foreground">Active filters:</span>
          {filter.method && (
            <span className="bg-primary/10 text-gray-900 dark:text-gray-100 px-2 py-1 rounded text-xs">
              Method: {filter.method}
            </span>
          )}
          {filter.status_code && (
            <span className="bg-primary/10 text-gray-900 dark:text-gray-100 px-2 py-1 rounded text-xs">
              Status: {filter.status_code}
            </span>
          )}
          {filter.level && (
            <span className="bg-primary/10 text-gray-900 dark:text-gray-100 px-2 py-1 rounded text-xs">
              Level: {filter.level}
            </span>
          )}
          {filter.path_pattern && (
            <span className="bg-primary/10 text-gray-900 dark:text-gray-100 px-2 py-1 rounded text-xs">
              Search: "{filter.path_pattern}"
            </span>
          )}
        </div>
      )}
    </div>
  );
}
