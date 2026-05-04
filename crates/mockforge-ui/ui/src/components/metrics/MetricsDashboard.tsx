import { logger } from '@/utils/logger';
import React, { useEffect } from 'react';
import { LatencyHistogram } from './LatencyHistogram';
import { FailureCounter } from './FailureCounter';
import { Button } from '../ui/button';
import { useMetricsStore } from '../../stores/useMetricsStore';

export function MetricsDashboard() {
  const {
    latencyMetrics,
    failureMetrics,
    selectedService,
    isLoading,
    lastUpdated,
    setSelectedService,
    refreshMetrics,
  } = useMetricsStore();

  useEffect(() => {
    // Initial load
    refreshMetrics();
  }, [refreshMetrics]);

  const handleServiceChange = (service: string) => {
    setSelectedService(service || null);
  };

  const formatLastUpdated = () => {
    if (!lastUpdated) return 'Never';
    const now = new Date();
    const diff = now.getTime() - lastUpdated.getTime();
    const seconds = Math.floor(diff / 1000);

    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ago`;
  };

  const getOverallStats = () => {
    const totalRequests = failureMetrics.reduce((sum, metric) => sum + (metric.total_requests || 0), 0);
    const totalFailures = failureMetrics.reduce((sum, metric) => sum + (metric.failure_count || 0), 0);
    const overallErrorRate = totalRequests > 0 ? totalFailures / totalRequests : 0;

    const avgP50 = latencyMetrics.length > 0
      ? Math.round(latencyMetrics.reduce((sum, metric) => sum + (metric.p50 || 0), 0) / latencyMetrics.length)
      : 0;

    const avgP95 = latencyMetrics.length > 0
      ? Math.round(latencyMetrics.reduce((sum, metric) => sum + (metric.p95 || 0), 0) / latencyMetrics.length)
      : 0;

    return {
      totalRequests,
      totalFailures,
      overallErrorRate,
      avgP50,
      avgP95,
    };
  };

  const stats = getOverallStats();

  return (
    <div className="space-y-6 p-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">Performance Metrics</h2>
          <p className="text-muted-foreground">
            Real-time performance monitoring and failure analysis
          </p>
        </div>

        <div className="flex items-center space-x-4">
          <div className="text-sm text-muted-foreground">
            Last updated: {formatLastUpdated()}
          </div>
          <Button
            onClick={refreshMetrics}
            disabled={isLoading}
            size="sm"
          >
            {isLoading ? 'Refreshing...' : 'Refresh'}
          </Button>
        </div>
      </div>

      {/* Overall Stats */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
        <div className="rounded-lg border bg-card p-4 text-center">
          <div className="text-2xl font-bold">{stats.totalRequests.toLocaleString()}</div>
          <div className="text-xs text-muted-foreground">Total Requests</div>
        </div>
        <div className="rounded-lg border bg-card p-4 text-center">
          <div className={`text-2xl font-bold ${stats.overallErrorRate < 0.05 ? 'text-success-600' : stats.overallErrorRate < 0.1 ? 'text-warning-600' : 'text-danger-600'}`}>
            {(stats.overallErrorRate * 100).toFixed(2)}%
          </div>
          <div className="text-xs text-muted-foreground">Error Rate</div>
        </div>
        <div className="rounded-lg border bg-card p-4 text-center">
          <div className="text-2xl font-bold text-success-600">{stats.avgP50}ms</div>
          <div className="text-xs text-muted-foreground">Avg P50</div>
        </div>
        <div className="rounded-lg border bg-card p-4 text-center">
          <div className="text-2xl font-bold text-warning-600">{stats.avgP95}ms</div>
          <div className="text-xs text-muted-foreground">Avg P95</div>
        </div>
        <div className="rounded-lg border bg-card p-4 text-center">
          <div className="text-2xl font-bold">{latencyMetrics.length}</div>
          <div className="text-xs text-muted-foreground">Services</div>
        </div>
      </div>

      {/* Latency Histogram */}
      <LatencyHistogram
        metrics={latencyMetrics}
        selectedService={selectedService || undefined}
        onServiceChange={handleServiceChange}
      />

      {/* Failure Analysis */}
      <FailureCounter
        metrics={failureMetrics}
        selectedService={selectedService || undefined}
        onServiceChange={handleServiceChange}
      />

      {/* SLA Status */}
      <div className="rounded-lg border bg-card p-6">
        <h3 className="text-lg font-semibold mb-4">SLA Status</h3>
        <div className="space-y-4">
          {latencyMetrics.map((metric) => {
            const slaP95 = 500; // 500ms SLA
            const slaErrorRate = 0.05; // 5% error rate SLA
            const failureMetric = failureMetrics.find(f => f.service === metric.service);

            const p95Status = (metric.p95 || 0) <= slaP95;
            const errorRateStatus = failureMetric ? (failureMetric.error_rate || 0) <= slaErrorRate : true;
            const overallStatus = p95Status && errorRateStatus;

            return (
              <div key={metric.service} className="flex items-center justify-between p-3 border rounded">
                <div className="flex items-center space-x-3">
                  <div className={`w-3 h-3 rounded-full ${overallStatus ? 'bg-success-500' : 'bg-danger-500'}`} />
                  <span className="font-medium">{metric.service}</span>
                </div>
                <div className="flex items-center space-x-6 text-sm">
                  <div className={`flex items-center space-x-1 ${p95Status ? 'text-success-600' : 'text-danger-600'}`}>
                    <span>P95: {metric.p95 || 0}ms</span>
                    <span>{p95Status ? '✓' : '✗'}</span>
                  </div>
                  <div className={`flex items-center space-x-1 ${errorRateStatus ? 'text-success-600' : 'text-danger-600'}`}>
                    <span>Error Rate: {failureMetric ? ((failureMetric.error_rate || 0) * 100).toFixed(2) : '0.00'}%</span>
                    <span>{errorRateStatus ? '✓' : '✗'}</span>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Alert Indicators */}
      {stats.overallErrorRate > 0.1 && (
        <div className="rounded-lg border border-danger-200 bg-danger-50 p-4">
          <div className="flex items-center space-x-2">
            <div className="text-danger-600">⚠️</div>
            <div>
              <div className="font-semibold text-danger-700">High Error Rate Alert</div>
              <div className="text-sm text-danger-700">
                Overall error rate is {(stats.overallErrorRate * 100).toFixed(2)}%, which exceeds the 10% threshold.
              </div>
            </div>
          </div>
        </div>
      )}

      {stats.avgP95 > 1000 && (
        <div className="rounded-lg border border-warning-200 bg-warning-50 p-4">
          <div className="flex items-center space-x-2">
            <div className="text-warning-600">⚠️</div>
            <div>
              <div className="font-semibold text-warning-700">High Latency Alert</div>
              <div className="text-sm text-warning-700">
                Average P95 latency is {stats.avgP95}ms, which may impact user experience.
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
