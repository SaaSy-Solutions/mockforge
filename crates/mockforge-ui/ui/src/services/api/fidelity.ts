import { fetchJsonWithErrorText } from './client';

export interface FieldDelta {
  location: string;
  change: string;
  mock?: unknown;
  source?: unknown;
}

export interface FidelityAssessment {
  definition: {
    format_version: number;
    routes_with_drift: number;
    routes: Array<{ endpoint: string; status: string; change: string; deltas: FieldDelta[] }>;
  };
  sample: null | {
    exchanges_checked: number;
    latest_capture_at: string;
    routes: Array<{ endpoint: string; status: string; exchanges_checked: number; findings: string[]; notes: string[] }>;
  };
}

export async function compareFidelity(
  mockSpec: string, sourceSpec: string, sample: unknown[] | undefined, signal: AbortSignal,
): Promise<FidelityAssessment> {
  const data = await fetchJsonWithErrorText('/__mockforge/fidelity/compare', {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ mock_spec: mockSpec, source_spec: sourceSpec, sample }), signal,
  }) as FidelityAssessment;
  if (!data?.definition || !Array.isArray(data.definition.routes)
    || (data.sample != null && !Array.isArray(data.sample.routes))) {
    throw new Error('A fidelity report was not returned. Check the admin server and try again.');
  }
  return data;
}

export function parseRecorderSample(text: string): unknown[] | undefined {
  if (!text.trim()) return undefined;
  try {
    const parsed: unknown = JSON.parse(text);
    if (Array.isArray(parsed)) return parsed;
    if (parsed && typeof parsed === 'object' && 'request' in parsed) return [parsed];
  } catch {
    try { return text.trim().split(/\r?\n/).filter(line => line.trim()).map(line => JSON.parse(line)); }
    catch { /* Show the recorder-specific recovery below. */ }
  }
  throw new Error('Use a Flight Recorder JSONL export or a JSON array of request/response exchanges.');
}
