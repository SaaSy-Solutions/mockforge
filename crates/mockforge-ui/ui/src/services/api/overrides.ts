/**
 * Response override rules for the running mock.
 *
 * Self-hosted: the admin server's `/__mockforge/overrides`.
 * Cloud: the registry's `/api/v1/hosted-mocks/{id}/overrides`, which stores
 * the rules on the deployment and pushes them to its running machine.
 */
import { fetchJsonWithErrorBody, fetchJsonWithErrorText } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export type PatchOp =
  | { op: 'add' | 'replace'; path: string; value: unknown }
  | { op: 'remove'; path: string };

export interface OverrideRule {
  name?: string;
  enabled: boolean;
  targets: string[];
  patch: PatchOp[];
  when?: string;
  mode: 'replace' | 'merge';
  post_templating: boolean;
}

/** Whether the running mock picked up a save. */
export type RuntimeSync = 'applied' | 'outdated' | 'unreachable';

export interface SaveResult {
  rules: OverrideRule[];
  runtime: RuntimeSync;
}

export interface HostedMockSummary {
  id: string;
  name: string;
  status: string;
}

function endpoint(deploymentId: string | null): string {
  if (!isCloudMode()) return '/__mockforge/overrides';
  if (!deploymentId) throw new Error('Choose a hosted mock first.');
  return `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/overrides`;
}

export const overridesApi = {
  async listHostedMocks(): Promise<HostedMockSummary[]> {
    const rows = await fetchJsonWithErrorBody('/api/v1/hosted-mocks');
    return Array.isArray(rows) ? (rows as HostedMockSummary[]) : [];
  },

  async list(deploymentId: string | null): Promise<OverrideRule[]> {
    const body = (await fetchJsonWithErrorText(endpoint(deploymentId))) as { rules?: OverrideRule[] };
    return body.rules ?? [];
  },

  async save(deploymentId: string | null, rules: OverrideRule[]): Promise<SaveResult> {
    const body = (await fetchJsonWithErrorText(endpoint(deploymentId), {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ rules }),
    })) as { rules: OverrideRule[]; runtime?: RuntimeSync };
    return { rules: body.rules, runtime: body.runtime ?? 'applied' };
  },
};
