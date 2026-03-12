/** Conformance run status */
export type RunStatus = 'pending' | 'running' | 'completed' | 'failed';

/** Request body for starting a conformance run */
export interface ConformanceRunRequest {
  target_url: string;
  spec?: string;
  categories?: string[];
  custom_headers?: [string, string][];
  api_key?: string;
  basic_auth?: string;
  skip_tls_verify?: boolean;
  base_path?: string;
  all_operations?: boolean;
  custom_checks_yaml?: string;
}

/** A conformance test run */
export interface ConformanceRun {
  id: string;
  status: RunStatus;
  config: ConformanceRunRequest;
  report?: ConformanceReport;
  error?: string;
  checks_done: number;
  total_checks: number;
}

/** Summary of a conformance run (from list endpoint) */
export interface ConformanceRunSummary {
  id: string;
  status: RunStatus;
  checks_done: number;
  total_checks: number;
  target_url: string;
}

/** Conformance report returned after completion */
export interface ConformanceReport {
  summary: ReportSummary;
  categories: Record<string, CategoryResult>;
  failures: FailureDetail[];
  owasp_coverage?: Record<string, string[]>;
}

/** Overall summary statistics */
export interface ReportSummary {
  total_checks: number;
  passed: number;
  failed: number;
  overall_rate: number;
}

/** Per-category results */
export interface CategoryResult {
  passed: number;
  total: number;
  rate: number;
}

/** Details about a failed check */
export interface FailureDetail {
  check_name: string;
  category: string;
  expected: string;
  actual: string;
  details?: string;
}

/** SSE progress event types */
export type ConformanceProgress =
  | { type: 'started'; total_checks: number }
  | { type: 'check_completed'; name: string; passed: boolean; checks_done: number }
  | { type: 'finished' }
  | { type: 'error'; message: string };
