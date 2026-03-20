/**
 * Contract Diff API service — capture uploads, analysis, statistics, patch generation.
 */
import { fetchJsonWithErrorText, authenticatedFetch } from './client';

// Contract Diff API types
export interface CapturedRequest {
  id?: string;
  method: string;
  path: string;
  source: string;
  captured_at?: string;
  analyzed?: boolean;
  query_params?: Record<string, string>;
  headers?: Record<string, string>;
  body?: unknown;
  status_code?: number;
  response_body?: unknown;
}

export interface ContractDiffResult {
  matches: boolean;
  confidence: number;
  mismatches: Mismatch[];
  recommendations: Recommendation[];
  corrections: CorrectionProposal[];
  metadata?: {
    contract_format?: string;
    analyzed_at?: string;
  };
}

export interface Mismatch {
  path: string;
  description: string;
  mismatch_type: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  confidence: number;
  expected?: string;
  actual?: string;
  context?: {
    is_additive?: boolean;
    is_breaking?: boolean;
    change_category?: string;
    schema_format?: string;
    service?: string;
    method?: string;
    field_name?: string;
    old_type?: string;
    new_type?: string;
    [key: string]: unknown;
  };
}

export interface Recommendation {
  recommendation: string;
  suggested_fix?: string;
  confidence: number;
}

export interface CorrectionProposal {
  description: string;
  path: string;
  operation: 'add' | 'remove' | 'replace';
  value?: unknown;
  confidence: number;
}

export interface CaptureStatistics {
  total_captures: number;
  analyzed_captures: number;
  sources: Record<string, number>;
  methods: Record<string, number>;
}

export interface AnalyzeRequestPayload {
  spec_path?: string;
  spec_content?: string;
  contract_id?: string;
  config?: {
    llm_provider?: string;
    llm_model?: string;
    confidence_threshold?: number;
  };
}

class ContractDiffApiService {
  async uploadRequest(request: Omit<CapturedRequest, 'id' | 'captured_at' | 'analyzed'>): Promise<{ capture_id: string; message: string }> {
    const response = await authenticatedFetch('/__mockforge/contract-diff/upload', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
    const json = await response.json();
    if (!response.ok) {
      throw new Error(json.error || `HTTP error! status: ${response.status}`);
    }
    return json;
  }

  async getCapturedRequests(params?: {
    source?: string;
    method?: string;
    path_pattern?: string;
    analyzed?: boolean;
    limit?: number;
    offset?: number;
  }): Promise<{ count: number; captures: CapturedRequest[] }> {
    const queryParams = new URLSearchParams();
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
          queryParams.append(key, String(value));
        }
      });
    }
    const url = `/__mockforge/contract-diff/captures${queryParams.toString() ? `?${queryParams}` : ''}`;
    return fetchJsonWithErrorText(url) as Promise<{ count: number; captures: CapturedRequest[] }>;
  }

  async getCapturedRequest(id: string): Promise<{ capture: CapturedRequest }> {
    return fetchJsonWithErrorText(`/__mockforge/contract-diff/captures/${id}`) as Promise<{ capture: CapturedRequest }>;
  }

  async analyzeCapturedRequest(id: string, payload: AnalyzeRequestPayload): Promise<{ analysis_result_id: string; result: ContractDiffResult }> {
    return fetchJsonWithErrorText(`/__mockforge/contract-diff/captures/${id}/analyze`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    }) as Promise<{ analysis_result_id: string; result: ContractDiffResult }>;
  }

  async getStatistics(): Promise<{ statistics: CaptureStatistics }> {
    return fetchJsonWithErrorText('/__mockforge/contract-diff/statistics') as Promise<{ statistics: CaptureStatistics }>;
  }

  async generatePatchFile(id: string, payload: AnalyzeRequestPayload): Promise<{ patch_file: unknown; corrections_count: number }> {
    return fetchJsonWithErrorText(`/__mockforge/contract-diff/captures/${id}/patch`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    }) as Promise<{ patch_file: unknown; corrections_count: number }>;
  }
}

export { ContractDiffApiService };
