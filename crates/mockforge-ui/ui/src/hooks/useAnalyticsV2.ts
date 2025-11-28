/**
 * Hook for accessing Analytics V2 API
 * Provides integration with the new persistent analytics database
 */

import { useQuery, UseQueryResult } from '@tanstack/react-query';

// API Base URL
const API_BASE = '/api/v2/analytics';

// ============================================================================
// Types
// ============================================================================

export interface OverviewMetrics {
  total_requests: number;
  total_errors: number;
  error_rate: number;
  avg_latency_ms: number;
  p95_latency_ms: number;
  p99_latency_ms: number;
  active_connections: number;
  total_bytes_sent: number;
  total_bytes_received: number;
  requests_per_second: number;
  top_protocols: ProtocolStat[];
  top_endpoints: EndpointStat[];
}

export interface ProtocolStat {
  protocol: string;
  request_count: number;
  error_count: number;
  avg_latency_ms: number;
}

export interface EndpointStat {
  endpoint: string;
  protocol: string;
  method: string | null;
  request_count: number;
  error_count: number;
  error_rate: number;
  avg_latency_ms: number;
  p95_latency_ms: number;
}

export interface TimeSeriesData {
  series: SeriesData[];
}

export interface SeriesData {
  label: string;
  data: DataPoint[];
}

export interface DataPoint {
  timestamp: number;
  value: number;
}

export interface LatencyTrendData {
  timestamp: number;
  p50: number;
  p95: number;
  p99: number;
  avg: number;
  min: number;
  max: number;
}

export interface ErrorSummaryData {
  error_type: string;
  error_category: string;
  count: number;
  endpoints: string[];
  last_occurrence: string;
}

export interface EndpointData {
  endpoint: string;
  protocol: string;
  method: string | null;
  total_requests: number;
  total_errors: number;
  error_rate: number;
  avg_latency_ms: number;
  p95_latency_ms: number;
  bytes_sent: number;
  bytes_received: number;
}

export interface TrafficPatternData {
  date: string;
  hour: number;
  day_of_week: number;
  request_count: number;
  error_count: number;
  avg_latency_ms: number;
}

export interface AnalyticsFilter {
  start_time?: number;
  end_time?: number;
  duration?: number;
  protocol?: string;
  endpoint?: string;
  method?: string;
  status_code?: number;
  workspace_id?: string;
  environment?: string;
  limit?: number;
  granularity?: 'minute' | 'hour' | 'day';
}

interface ApiResponse<T> {
  success: boolean;
  data: T;
  error?: string;
}

// ============================================================================
// Helper Functions
// ============================================================================

function buildQueryString(params: Record<string, any>): string {
  const filtered = Object.entries(params).filter(
    ([_, value]) => value !== undefined && value !== null && value !== ''
  );
  if (filtered.length === 0) return '';
  const queryParams = new URLSearchParams(
    filtered.map(([key, value]) => [key, String(value)])
  );
  return `?${queryParams.toString()}`;
}

async function fetchAnalytics<T>(endpoint: string, filter?: AnalyticsFilter): Promise<T> {
  const queryString = filter ? buildQueryString(filter) : '';
  const response = await fetch(`${API_BASE}${endpoint}${queryString}`);

  if (!response.ok) {
    throw new Error(`Analytics API error: ${response.statusText}`);
  }

  const json: ApiResponse<T> = await response.json();

  if (!json.success) {
    throw new Error(json.error || 'Unknown error');
  }

  return json.data;
}

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Get overview metrics for the dashboard
 */
export function useOverviewMetrics(
  filter?: AnalyticsFilter,
  options?: { refetchInterval?: number }
): UseQueryResult<OverviewMetrics> {
  return useQuery({
    queryKey: ['analytics', 'overview', filter],
    queryFn: () => fetchAnalytics<OverviewMetrics>('/overview', filter),
    refetchInterval: options?.refetchInterval,
  });
}

/**
 * Get request count time-series data
 */
export function useRequestTimeSeries(
  filter?: AnalyticsFilter
): UseQueryResult<TimeSeriesData> {
  return useQuery({
    queryKey: ['analytics', 'requests', filter],
    queryFn: () => fetchAnalytics<TimeSeriesData>('/requests', filter),
  });
}

/**
 * Get latency trends (percentiles over time)
 */
export function useLatencyTrends(
  filter?: AnalyticsFilter
): UseQueryResult<{ trends: LatencyTrendData[] }> {
  return useQuery({
    queryKey: ['analytics', 'latency', filter],
    queryFn: () =>
      fetchAnalytics<{ trends: LatencyTrendData[] }>('/latency', filter),
  });
}

/**
 * Get error summary
 */
export function useErrorSummary(
  filter?: AnalyticsFilter
): UseQueryResult<{ errors: ErrorSummaryData[] }> {
  return useQuery({
    queryKey: ['analytics', 'errors', filter],
    queryFn: () =>
      fetchAnalytics<{ errors: ErrorSummaryData[] }>('/errors', filter),
  });
}

/**
 * Get top endpoints by traffic
 */
export function useTopEndpoints(
  filter?: AnalyticsFilter
): UseQueryResult<{ endpoints: EndpointData[] }> {
  return useQuery({
    queryKey: ['analytics', 'endpoints', filter],
    queryFn: () =>
      fetchAnalytics<{ endpoints: EndpointData[] }>('/endpoints', filter),
  });
}

/**
 * Get protocol breakdown
 */
export function useProtocolBreakdown(
  filter?: AnalyticsFilter
): UseQueryResult<{ protocols: ProtocolStat[] }> {
  return useQuery({
    queryKey: ['analytics', 'protocols', filter],
    queryFn: () =>
      fetchAnalytics<{ protocols: ProtocolStat[] }>('/protocols', filter),
  });
}

/**
 * Get traffic patterns for heatmap
 */
export function useTrafficPatterns(
  days: number = 30,
  workspace_id?: string
): UseQueryResult<{ patterns: TrafficPatternData[] }> {
  return useQuery({
    queryKey: ['analytics', 'traffic-patterns', days, workspace_id],
    queryFn: () =>
      fetchAnalytics<{ patterns: TrafficPatternData[] }>(
        '/traffic-patterns',
        { limit: days, workspace_id } as any
      ),
  });
}

// ============================================================================
// Export Functions
// ============================================================================

/**
 * Export analytics data to CSV
 */
export async function exportToCSV(filter?: AnalyticsFilter): Promise<string> {
  const queryString = filter ? buildQueryString(filter) : '';
  const response = await fetch(`${API_BASE}/export/csv${queryString}`);

  if (!response.ok) {
    throw new Error(`Export failed: ${response.statusText}`);
  }

  return await response.text();
}

/**
 * Export analytics data to JSON
 */
export async function exportToJSON(filter?: AnalyticsFilter): Promise<string> {
  const queryString = filter ? buildQueryString(filter) : '';
  const response = await fetch(`${API_BASE}/export/json${queryString}`);

  if (!response.ok) {
    throw new Error(`Export failed: ${response.statusText}`);
  }

  return await response.text();
}

/**
 * Download exported data as a file
 */
export function downloadFile(content: string, filename: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
