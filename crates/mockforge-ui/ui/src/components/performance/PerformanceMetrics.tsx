/**
 * Performance Metrics Dashboard
 *
 * Displays real-time performance metrics including:
 * - RPS (current vs target)
 * - Latency statistics (avg, P95, P99)
 * - Error rate
 * - Request counts
 */

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { usePerformanceSnapshot } from '../../hooks/usePerformance';
import { Activity, TrendingUp, AlertCircle, Clock, Zap } from 'lucide-react';

export function PerformanceMetrics() {
  const { data: snapshot, isLoading, error } = usePerformanceSnapshot();

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {[1, 2, 3, 4].map((i) => (
          <Card key={i}>
            <CardContent className="p-6">
              <div className="animate-pulse">
                <div className="h-4 bg-gray-200 rounded w-1/2 mb-2"></div>
                <div className="h-8 bg-gray-200 rounded w-3/4"></div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  if (error || !snapshot) {
    return (
      <Card>
        <CardContent className="p-6">
          <div className="text-center text-muted-foreground">
            {error ? 'Failed to load metrics' : 'Performance mode not started'}
          </div>
        </CardContent>
      </Card>
    );
  }

  const { metrics } = snapshot;
  const rpsDiff = metrics.current_rps - metrics.target_rps;
  const rpsDiffPercent = metrics.target_rps > 0
    ? ((rpsDiff / metrics.target_rps) * 100).toFixed(1)
    : '0';

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {/* Current RPS */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Current RPS</CardTitle>
          <Activity className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">{metrics.current_rps.toFixed(1)}</div>
          <p className="text-xs text-muted-foreground">
            Target: {metrics.target_rps.toFixed(1)} RPS
            {rpsDiff !== 0 && (
              <span className={rpsDiff > 0 ? 'text-red-500' : 'text-green-500'}>
                {' '}({rpsDiff > 0 ? '+' : ''}{rpsDiffPercent}%)
              </span>
            )}
          </p>
        </CardContent>
      </Card>

      {/* Average Latency */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Avg Latency</CardTitle>
          <Clock className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">{metrics.avg_latency_ms.toFixed(1)}ms</div>
          <p className="text-xs text-muted-foreground">
            P95: {metrics.p95_latency_ms}ms | P99: {metrics.p99_latency_ms}ms
          </p>
        </CardContent>
      </Card>

      {/* Error Rate */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Error Rate</CardTitle>
          <AlertCircle className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {(metrics.error_rate * 100).toFixed(2)}%
          </div>
          <p className="text-xs text-muted-foreground">
            {metrics.failed_requests} of {metrics.total_requests} requests
          </p>
        </CardContent>
      </Card>

      {/* Total Requests */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">Total Requests</CardTitle>
          <Zap className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">{metrics.total_requests.toLocaleString()}</div>
          <p className="text-xs text-muted-foreground">
            {metrics.successful_requests.toLocaleString()} successful
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
