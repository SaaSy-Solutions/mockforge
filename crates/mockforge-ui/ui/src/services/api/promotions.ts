/**
 * Scenario promotions API service — cloud registry only.
 * Backend: crates/mockforge-registry-server/src/handlers/scenario_promotions.rs
 */
import { fetchJson } from './client';

export type PromotionStatus = 'pending' | 'approved' | 'rejected' | 'completed' | 'failed';

export interface ScenarioPromotion {
  id: string;
  scenario_id: string;
  scenario_version: string;
  workspace_id: string;
  from_environment: string;
  to_environment: string;
  promoted_by: string;
  approved_by: string | null;
  status: string;
  requires_approval: boolean;
  approval_required_reason: string | null;
  comments: string | null;
  approval_comments: string | null;
  completed_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface PromoteScenarioRequest {
  scenario_id: string;
  scenario_version: string;
  from_environment: string;
  to_environment: string;
  comments?: string;
}

export interface PromoteScenarioResponse {
  promotion_id: string;
  status: PromotionStatus;
  requires_approval: boolean;
  approval_reason: string | null;
  message: string;
}

export interface PromotionListResponse {
  promotions: ScenarioPromotion[];
}

export interface ApprovePromotionRequest {
  comments?: string;
}

export interface RejectPromotionRequest {
  reason: string;
}

export interface PromotionActionResponse {
  promotion_id: string;
  status: PromotionStatus;
  message: string;
}

const base = (workspaceId: string) => `/api/v1/workspaces/${workspaceId}`;

export const promotionsApi = {
  async list(workspaceId: string, status?: PromotionStatus): Promise<ScenarioPromotion[]> {
    const qs = status ? `?status=${encodeURIComponent(status)}` : '';
    const response = (await fetchJson(`${base(workspaceId)}/promotions${qs}`)) as PromotionListResponse;
    return response.promotions ?? [];
  },

  async promote(
    workspaceId: string,
    environment: string,
    request: PromoteScenarioRequest
  ): Promise<PromoteScenarioResponse> {
    return fetchJson(
      `${base(workspaceId)}/environments/${encodeURIComponent(environment)}/promote-scenario`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      }
    ) as Promise<PromoteScenarioResponse>;
  },

  async approve(
    workspaceId: string,
    promotionId: string,
    request: ApprovePromotionRequest
  ): Promise<PromotionActionResponse> {
    return fetchJson(`${base(workspaceId)}/promotions/${promotionId}/approve`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<PromotionActionResponse>;
  },

  async reject(
    workspaceId: string,
    promotionId: string,
    request: RejectPromotionRequest
  ): Promise<PromotionActionResponse> {
    return fetchJson(`${base(workspaceId)}/promotions/${promotionId}/reject`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<PromotionActionResponse>;
  },
};
