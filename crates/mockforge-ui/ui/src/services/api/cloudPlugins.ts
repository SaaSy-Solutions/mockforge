/**
 * Cloud Plugins API client.
 *
 * Two surfaces:
 *   1. Beta-interest CTA (Phase 0 — `/api/v1/cloud-plugins/beta-interest`).
 *      Per-user, not org-scoped.
 *   2. Per-deployment plugin attachments (Phase 1 control-plane —
 *      `/api/v1/hosted-mocks/{deployment_id}/plugins`). Org from
 *      `X-Organization-Id` is set on the request by `authenticatedFetch`
 *      / the registry's auth middleware; no header needed here.
 *
 * The attachment endpoints land in PR #395; until that merges they
 * return 404 and `listAttachments` swallows that into an empty list so
 * the page renders cleanly during the gap.
 */
import { fetchJsonWithErrorBody } from './client';
import { authenticatedFetch } from '../../utils/apiClient';

// ─── Beta interest (Phase 0) ───────────────────────────────────────────

export interface BetaInterestStatus {
  signed_up: boolean;
  created_at?: string;
  use_case?: string;
}

export interface BetaInterestSubmission {
  id: string;
  created_at: string;
  updated_at: string;
}

export interface SubmitBetaInterestRequest {
  use_case?: string;
}

// ─── Plugin attachments (Phase 1, PR #395) ─────────────────────────────

/**
 * Permission grant payload — structured per RFC §4.2. The shape is
 * intentionally open here; the editor in 3.3 will narrow each section
 * to typed inputs. Today the UI just round-trips whatever the backend
 * returns.
 */
export type PermissionsGrant = Record<string, unknown>;

export type PluginConfig = Record<string, unknown>;

export interface PluginAttachment {
  id: string;
  deployment_id: string;
  plugin_id: string;
  plugin_version_id: string;
  /** Joined fields from `plugins` / `plugin_versions` for display. */
  plugin_name?: string;
  plugin_version?: string;
  config_json: PluginConfig;
  permissions_json: PermissionsGrant;
  enabled: boolean;
  attached_at: string;
  updated_at: string;
  attached_by?: string | null;
}

export interface AttachPluginRequest {
  plugin_name: string;
  plugin_version: string;
  config_json?: PluginConfig;
  /** Empty object = deny-all (RFC §4.2 default). */
  permissions_json?: PermissionsGrant;
}

export interface UpdateAttachmentRequest {
  enabled?: boolean;
  config_json?: PluginConfig;
  permissions_json?: PermissionsGrant;
}

export const cloudPluginsApi = {
  // ── Beta interest ────────────────────────────────────────────────
  async getMyBetaInterest(): Promise<BetaInterestStatus> {
    return (await fetchJsonWithErrorBody(
      '/api/v1/cloud-plugins/beta-interest/me',
    )) as BetaInterestStatus;
  },

  async submitBetaInterest(
    body: SubmitBetaInterestRequest,
  ): Promise<BetaInterestSubmission> {
    return (await fetchJsonWithErrorBody('/api/v1/cloud-plugins/beta-interest', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })) as BetaInterestSubmission;
  },

  // ── Per-deployment attachments ───────────────────────────────────
  /**
   * GET /api/v1/hosted-mocks/{deployment_id}/plugins
   *
   * Returns 404 when the control-plane API isn't deployed yet (pre PR
   * #395 merge). We translate 404 into an empty list so the page can
   * render without an error banner — the absence of attachments is
   * indistinguishable from "feature not yet live" to the end user.
   */
  async listAttachments(deploymentId: string): Promise<PluginAttachment[]> {
    const response = await authenticatedFetch(
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/plugins`,
    );
    if (response.status === 404) {
      return [];
    }
    if (!response.ok) {
      throw new Error(`Failed to list attachments (HTTP ${response.status})`);
    }
    const json = await response.json();
    const data = json.data ?? json;
    return Array.isArray(data) ? (data as PluginAttachment[]) : [];
  },

  async attachPlugin(
    deploymentId: string,
    body: AttachPluginRequest,
  ): Promise<PluginAttachment> {
    return (await fetchJsonWithErrorBody(
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/plugins`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    )) as PluginAttachment;
  },

  async updateAttachment(
    deploymentId: string,
    attachmentId: string,
    body: UpdateAttachmentRequest,
  ): Promise<PluginAttachment> {
    return (await fetchJsonWithErrorBody(
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/plugins/${encodeURIComponent(attachmentId)}`,
      {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    )) as PluginAttachment;
  },

  async detachPlugin(
    deploymentId: string,
    attachmentId: string,
  ): Promise<void> {
    const response = await authenticatedFetch(
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/plugins/${encodeURIComponent(attachmentId)}`,
      { method: 'DELETE' },
    );
    // DELETE is idempotent in #395 — 404 on a stale row is fine.
    if (!response.ok && response.status !== 404) {
      throw new Error(`Failed to detach plugin (HTTP ${response.status})`);
    }
  },
};
