import { z } from 'zod';

// ==================== LOG SCHEMAS ====================

export const RequestLogSchema = z.object({
  id: z.string(),
  timestamp: z.string(),
  method: z.string(),
  path: z.string(),
  status_code: z.number(),
  response_time_ms: z.number(),
  client_ip: z.string().nullable().optional(),
  user_agent: z.string().nullable().optional(),
  headers: z.record(z.string()).nullable().optional(),
  response_size_bytes: z.number(),
  request_size_bytes: z.number().nullable().optional(),
  error_message: z.string().nullable().optional(),
});

export const LogEntrySchema = z.object({
  timestamp: z.string(),
  status: z.number(),
  method: z.string(),
  url: z.string(),
  responseTime: z.number(),
  size: z.number(),
  status_code: z.number().nullable().optional(),
  response_time_ms: z.number().nullable().optional(),
});

// ==================== WORKSPACE SCHEMAS ====================

export const WorkspaceSummarySchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string().optional(),
  is_active: z.boolean().default(false),
  created_at: z.string().optional(),
  updated_at: z.string().optional(),
  route_count: z.number().optional(),
  fixture_count: z.number().optional(),
});

export const WorkspaceDetailsSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string().optional(),
  is_active: z.boolean().default(false),
  created_at: z.string().optional(),
  updated_at: z.string().optional(),
  fixtures: z.array(z.any()).optional(),
  routes: z.array(z.any()).optional(),
});

// ==================== FIXTURE SCHEMAS ====================

export const FixtureInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
  path: z.string(),
  method: z.string().optional(),
  description: z.string().optional(),
  createdAt: z.string(),
  updatedAt: z.string(),
  tags: z.array(z.string()).optional(),
  content: z.union([z.string(), z.unknown()]).optional(),
  version: z.string().optional(),
  size_bytes: z.number().optional(),
  last_modified: z.string().optional(),
  route_path: z.string().optional(),
  protocol: z.string().optional(),
  saved_at: z.string().optional(),
  fingerprint: z.string().optional(),
  metadata: z.record(z.unknown()).optional(),
  file_size: z.number().optional(),
  file_path: z.string().optional(),
  size: z.number().optional(),
  created_at: z.string().optional(),
  modified_at: z.string().optional(),
});

// ==================== SERVICE SCHEMAS ====================

export const ServiceInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
  status: z.enum(['active', 'inactive', 'error']),
  port: z.number().optional(),
  endpoint: z.string().optional(),
  description: z.string().optional(),
  uptime: z.number().optional(),
  request_count: z.number().optional(),
  error_rate: z.number().optional(),
});

// ==================== DASHBOARD SCHEMAS ====================

export const ServerInfoSchema = z.object({
  version: z.string(),
  build_time: z.string(),
  git_sha: z.string(),
  http_server: z.string().nullable().optional(),
  ws_server: z.string().nullable().optional(),
  grpc_server: z.string().nullable().optional(),
  graphql_server: z.string().nullable().optional(),
  api_enabled: z.boolean(),
  admin_port: z.number(),
});

export const DashboardSystemInfoSchema = z.object({
  os: z.string(),
  arch: z.string(),
  uptime: z.number(),
  memory_usage: z.number(),
});

export const SimpleMetricsDataSchema = z.object({
  total_requests: z.number(),
  active_requests: z.number(),
  average_response_time: z.number(),
  error_rate: z.number(),
});

export const ServerStatusSchema = z.object({
  server_type: z.string(),
  address: z.string().nullable().optional(),
  running: z.boolean(),
  start_time: z.string().nullable().optional(),
  uptime_seconds: z.number().nullable().optional(),
  active_connections: z.number(),
  total_requests: z.number(),
});

export const SystemInfoSchema = z.object({
  version: z.string(),
  uptime_seconds: z.number(),
  memory_usage_mb: z.number(),
  cpu_usage_percent: z.number(),
  active_threads: z.number(),
  total_routes: z.number(),
  total_fixtures: z.number(),
});

export const DashboardDataSchema = z.object({
  server_info: ServerInfoSchema,
  system_info: DashboardSystemInfoSchema,
  metrics: SimpleMetricsDataSchema,
  servers: z.array(ServerStatusSchema),
  recent_logs: z.array(RequestLogSchema),
  system: SystemInfoSchema,
}).passthrough();

// ==================== METRICS SCHEMAS ====================

export const LatencyMetricsSchema = z.object({
  service: z.string(),
  route: z.string(),
  avg_response_time: z.number(),
  min_response_time: z.number(),
  max_response_time: z.number(),
  p50_response_time: z.number(),
  p95_response_time: z.number(),
  p99_response_time: z.number(),
  total_requests: z.number(),
  histogram: z.array(z.object({
    range: z.string(),
    count: z.number(),
  })).optional(),
});

// ==================== RESPONSE SCHEMAS ====================

export const WorkspaceListResponseSchema = z.array(WorkspaceSummarySchema);

export const LogsResponseSchema = z.array(RequestLogSchema);

export const DashboardResponseSchema = DashboardDataSchema;

export const FixturesResponseSchema = z.array(FixtureInfoSchema);

export const ServicesResponseSchema = z.array(ServiceInfoSchema);

// ==================== VALIDATION HELPERS ====================

export function validateApiResponse<T>(schema: z.ZodSchema<T>, data: unknown): T {
  return schema.parse(data);
}

export function safeValidateApiResponse<T>(
  schema: z.ZodSchema<T>,
  data: unknown
): { success: true; data: T } | { success: false; error: z.ZodError } {
  try {
    const result = schema.safeParse(data);
    if (result.success) {
      return { success: true, data: result.data };
    }
    return { success: false, error: result.error };
  } catch (error) {
    console.error('[VALIDATION ERROR] Exception:', error);
    throw error;
  }
}

// ==================== TYPE EXPORTS ====================

export type RequestLog = z.infer<typeof RequestLogSchema>;
export type LogEntry = z.infer<typeof LogEntrySchema>;
export type WorkspaceSummary = z.infer<typeof WorkspaceSummarySchema>;
export type WorkspaceDetails = z.infer<typeof WorkspaceDetailsSchema>;
export type FixtureInfo = z.infer<typeof FixtureInfoSchema>;
export type ServiceInfo = z.infer<typeof ServiceInfoSchema>;
export type DashboardData = z.infer<typeof DashboardDataSchema>;
export type LatencyMetrics = z.infer<typeof LatencyMetricsSchema>;
