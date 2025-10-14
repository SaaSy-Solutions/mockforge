import React from 'react';
import { Protocol } from '../../../types';
import { MetricCard } from './MetricCard';
import { MetricIcon, ProtocolIcon } from '../ui/IconSystem';

export interface ProtocolMetrics {
  protocol: Protocol;
  totalRequests: number;
  successRate: number;
  averageLatency: number;
  errorRate: number;
  activeConnections?: number;
  throughput?: number; // requests per second
  customMetrics?: Record<string, number>;
}

export interface ProtocolDashboardProps {
  metrics: ProtocolMetrics[];
  selectedProtocol?: Protocol;
  onProtocolChange?: (protocol: Protocol) => void;
  className?: string;
}

export function ProtocolDashboard({
  metrics,
  selectedProtocol,
  onProtocolChange,
  className = ''
}: ProtocolDashboardProps) {
  const filteredMetrics = selectedProtocol
    ? metrics.filter(m => m.protocol === selectedProtocol)
    : metrics;

  const totalRequests = metrics.reduce((sum, m) => sum + m.totalRequests, 0);
  const averageSuccessRate = metrics.length > 0
    ? metrics.reduce((sum, m) => sum + m.successRate, 0) / metrics.length
    : 0;

  return (
    <div className={`space-y-6 ${className}`}>
      {/* Protocol Selector */}
      {onProtocolChange && (
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => onProtocolChange(undefined as any)}
            className={`px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
              !selectedProtocol
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'
            }`}
          >
            All Protocols
          </button>
          {metrics.map(metric => (
            <button
              key={metric.protocol}
              onClick={() => onProtocolChange(metric.protocol)}
              className={`px-3 py-2 rounded-lg text-sm font-medium transition-colors flex items-center gap-2 ${
                selectedProtocol === metric.protocol
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'
              }`}
            >
              <ProtocolIcon protocol={metric.protocol} size="sm" />
              {metric.protocol}
            </button>
          ))}
        </div>
      )}

      {/* Overall Metrics */}
      {!selectedProtocol && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <MetricCard
            title="Total Requests"
            value={totalRequests.toLocaleString()}
            subtitle="across all protocols"
            icon={<MetricIcon metric="requests" size="lg" />}
          />
          <MetricCard
            title="Average Success Rate"
            value={`${averageSuccessRate.toFixed(1)}%`}
            subtitle="across all protocols"
            icon={<MetricIcon metric="success" size="lg" />}
          />
          <MetricCard
            title="Active Protocols"
            value={metrics.length.toString()}
            subtitle="protocol types"
            icon={<MetricIcon metric="activity" size="lg" />}
          />
          <MetricCard
            title="Total Throughput"
            value={`${metrics.reduce((sum, m) => sum + (m.throughput || 0), 0).toFixed(1)} req/s`}
            subtitle="requests per second"
            icon={<MetricIcon metric="performance" size="lg" />}
          />
        </div>
      )}

      {/* Protocol-Specific Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {filteredMetrics.map(metric => (
          <ProtocolMetricsCard key={metric.protocol} metrics={metric} />
        ))}
      </div>

      {/* Custom Metrics */}
      {filteredMetrics.some(m => m.customMetrics && Object.keys(m.customMetrics).length > 0) && (
        <div className="space-y-4">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            Custom Metrics
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredMetrics
              .filter(m => m.customMetrics)
              .flatMap(m =>
                Object.entries(m.customMetrics!).map(([key, value]) => (
                  <MetricCard
                    key={`${m.protocol}-${key}`}
                    title={key.replace(/([A-Z])/g, ' $1').replace(/^./, str => str.toUpperCase())}
                    value={typeof value === 'number' && value % 1 !== 0 ? value.toFixed(2) : value.toString()}
                    subtitle={`${m.protocol} protocol`}
                    icon={<ProtocolIcon protocol={m.protocol} size="lg" />}
                  />
                ))
              )}
          </div>
        </div>
      )}
    </div>
  );
}

interface ProtocolMetricsCardProps {
  metrics: ProtocolMetrics;
}

function ProtocolMetricsCard({ metrics }: ProtocolMetricsCardProps) {
  return (
    <div className="bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-xl p-6">
      <div className="flex items-center gap-3 mb-4">
        <ProtocolIcon protocol={metrics.protocol} size="lg" />
        <div>
          <h3 className="font-semibold text-gray-900 dark:text-gray-100">
            {metrics.protocol}
          </h3>
          <p className="text-sm text-gray-600 dark:text-gray-400">
            Protocol Metrics
          </p>
        </div>
      </div>

      <div className="space-y-3">
        <div className="flex justify-between items-center">
          <span className="text-sm text-gray-600 dark:text-gray-400">Total Requests</span>
          <span className="font-medium text-gray-900 dark:text-gray-100">
            {metrics.totalRequests.toLocaleString()}
          </span>
        </div>

        <div className="flex justify-between items-center">
          <span className="text-sm text-gray-600 dark:text-gray-400">Success Rate</span>
          <span className={`font-medium ${
            metrics.successRate >= 95 ? 'text-green-600' :
            metrics.successRate >= 80 ? 'text-yellow-600' : 'text-red-600'
          }`}>
            {metrics.successRate.toFixed(1)}%
          </span>
        </div>

        <div className="flex justify-between items-center">
          <span className="text-sm text-gray-600 dark:text-gray-400">Avg Latency</span>
          <span className="font-medium text-gray-900 dark:text-gray-100">
            {metrics.averageLatency.toFixed(0)}ms
          </span>
        </div>

        <div className="flex justify-between items-center">
          <span className="text-sm text-gray-600 dark:text-gray-400">Error Rate</span>
          <span className={`font-medium ${
            metrics.errorRate <= 5 ? 'text-green-600' :
            metrics.errorRate <= 20 ? 'text-yellow-600' : 'text-red-600'
          }`}>
            {metrics.errorRate.toFixed(1)}%
          </span>
        </div>

        {metrics.activeConnections !== undefined && (
          <div className="flex justify-between items-center">
            <span className="text-sm text-gray-600 dark:text-gray-400">Active Connections</span>
            <span className="font-medium text-gray-900 dark:text-gray-100">
              {metrics.activeConnections}
            </span>
          </div>
        )}

        {metrics.throughput !== undefined && (
          <div className="flex justify-between items-center">
            <span className="text-sm text-gray-600 dark:text-gray-400">Throughput</span>
            <span className="font-medium text-gray-900 dark:text-gray-100">
              {metrics.throughput.toFixed(1)} req/s
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
