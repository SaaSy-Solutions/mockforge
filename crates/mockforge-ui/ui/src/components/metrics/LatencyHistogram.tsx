import React from 'react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import type { LatencyMetrics } from '../../types';

interface LatencyHistogramProps {
  metrics: LatencyMetrics[];
  selectedService?: string;
  onServiceChange: (service: string) => void;
}

export function LatencyHistogram({ metrics, selectedService, onServiceChange }: LatencyHistogramProps) {
  const selectedMetric = selectedService ? metrics.find(m => m.service === selectedService) : metrics[0];
  const histogramData = selectedMetric?.histogram || [];

  // Color bars based on latency ranges
  const getBarColor = (range: string) => {
    const numValue = parseInt(range.split('-')[0] || '0');
    if (numValue < 100) return '#10b981'; // green
    if (numValue < 500) return '#f59e0b'; // yellow
    if (numValue < 1000) return '#ef4444'; // red
    return '#dc2626'; // dark red
  };

  const formatTooltip = (value: number, name: string) => {
    if (name === 'count') {
      return [`${value} requests`, 'Count'];
    }
    return [value, name];
  };

  const formatPercentage = (percentage: number) => {
    return `${percentage.toFixed(1)}%`;
  };

  return (
    <div className="rounded-lg border bg-card p-6">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold">Response Time Distribution</h3>
        <select
          value={selectedService || ''}
          onChange={(e) => onServiceChange(e.target.value)}
          className="px-3 py-1 border border-input rounded text-sm bg-background"
        >
          <option value="">All Services</option>
          {metrics.map(metric => (
            <option key={metric.service} value={metric.service}>
              {metric.service}
            </option>
          ))}
        </select>
      </div>

      {selectedMetric && (
        <div className="mb-4">
          <div className="grid grid-cols-3 gap-4 text-center">
            <div className="space-y-1">
              <div className="text-2xl font-bold text-green-600">{selectedMetric.p50}ms</div>
              <div className="text-xs text-muted-foreground">P50 (Median)</div>
            </div>
            <div className="space-y-1">
              <div className="text-2xl font-bold text-yellow-600">{selectedMetric.p95}ms</div>
              <div className="text-xs text-muted-foreground">P95</div>
            </div>
            <div className="space-y-1">
              <div className="text-2xl font-bold text-red-600">{selectedMetric.p99}ms</div>
              <div className="text-xs text-muted-foreground">P99</div>
            </div>
          </div>
        </div>
      )}

      <div className="h-80">
        {histogramData.length > 0 ? (
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={histogramData} margin={{ top: 20, right: 30, left: 20, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" className="opacity-30" />
              <XAxis
                dataKey="range"
                angle={-45}
                textAnchor="end"
                height={80}
                fontSize={12}
              />
              <YAxis
                yAxisId="count"
                orientation="left"
                fontSize={12}
              />
              <YAxis
                yAxisId="percentage"
                orientation="right"
                fontSize={12}
                tickFormatter={formatPercentage}
              />
              <Tooltip
                formatter={formatTooltip}
                labelStyle={{ color: '#000' }}
                contentStyle={{
                  backgroundColor: 'white',
                  border: '1px solid #e2e8f0',
                  borderRadius: '6px',
                }}
              />
              <Bar yAxisId="count" dataKey="count" name="count">
                {histogramData.map((entry, index) => (
                  <Cell key={`cell-${index}`} fill={getBarColor(entry.range)} />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        ) : (
          <div className="flex items-center justify-center h-full text-center">
            <div className="space-y-2">
              <div className="text-4xl">📊</div>
              <div className="text-muted-foreground">No latency data available</div>
            </div>
          </div>
        )}
      </div>

      {selectedMetric && (
        <div className="mt-4 text-xs text-muted-foreground">
          <div className="flex justify-between">
            <span>Service: {selectedMetric.service}</span>
            <span>Route: {selectedMetric.route}</span>
          </div>
        </div>
      )}
    </div>
  );
}
