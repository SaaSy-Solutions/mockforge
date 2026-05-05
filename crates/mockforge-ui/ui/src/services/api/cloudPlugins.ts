/**
 * Cloud Plugins beta interest API (Phase 0 demand validation).
 *
 * Wraps `/api/v1/cloud-plugins/beta-interest{,/me}`. The endpoints are
 * per-user (not org-scoped) — registering interest is an individual
 * action, even if we snapshot the user's current org context server
 * side for analysis.
 */
import { fetchJsonWithErrorBody } from './client';

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

export const cloudPluginsApi = {
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
};
