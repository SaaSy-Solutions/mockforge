/**
 * Notification channels + routing rules (#3).
 *
 * Wraps `/api/v1/organizations/{org_id}/notification-channels` and
 * `/api/v1/organizations/{org_id}/routing-rules` plus the test-fire
 * + channel toggle endpoints.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export type NotificationChannelKind = 'email' | 'slack' | 'pagerduty' | 'webhook';

export interface NotificationChannel {
  id: string;
  org_id: string;
  name: string;
  kind: NotificationChannelKind;
  config: Record<string, unknown>;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateChannelRequest {
  name: string;
  kind: NotificationChannelKind;
  config: Record<string, unknown>;
  enabled?: boolean;
}

export interface UpdateChannelRequest {
  name?: string;
  config?: Record<string, unknown>;
  enabled?: boolean;
}

export interface TestFireResult {
  ok: boolean;
  kind: string;
  status_code?: number;
  error?: string;
  skipped?: boolean;
  reason?: string;
}

export interface RoutingRule {
  id: string;
  org_id: string;
  priority: number;
  match_severity: string[];
  match_source: string[];
  match_workspace_id: string | null;
  channel_ids: string[];
  created_at: string;
  updated_at: string;
}

export interface CreateRoutingRuleRequest {
  priority: number;
  match_severity?: string[];
  match_source?: string[];
  match_workspace_id?: string;
  channel_ids: string[];
}

class CloudNotificationsApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud notifications ${method} only works in cloud mode.`);
    }
  }

  // --- channels ------------------------------------------------------------

  async listChannels(orgId: string): Promise<NotificationChannel[]> {
    this.guard('listChannels');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/notification-channels`,
    ) as Promise<NotificationChannel[]>;
  }

  async createChannel(
    orgId: string,
    body: CreateChannelRequest,
  ): Promise<NotificationChannel> {
    this.guard('createChannel');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/notification-channels`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<NotificationChannel>;
  }

  async updateChannel(
    orgId: string,
    id: string,
    body: UpdateChannelRequest,
  ): Promise<NotificationChannel> {
    this.guard('updateChannel');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/notification-channels/${id}`,
      {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<NotificationChannel>;
  }

  async deleteChannel(
    orgId: string,
    id: string,
  ): Promise<{ deleted: boolean }> {
    this.guard('deleteChannel');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/notification-channels/${id}`,
      { method: 'DELETE' },
    ) as Promise<{ deleted: boolean }>;
  }

  /** Synthetic dispatch through this channel. No incident written. */
  async testFire(orgId: string, id: string): Promise<TestFireResult> {
    this.guard('testFire');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/notification-channels/${id}/test-fire`,
      { method: 'POST' },
    ) as Promise<TestFireResult>;
  }

  // --- routing rules -------------------------------------------------------

  async listRoutingRules(orgId: string): Promise<RoutingRule[]> {
    this.guard('listRoutingRules');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/routing-rules`,
    ) as Promise<RoutingRule[]>;
  }

  async createRoutingRule(
    orgId: string,
    body: CreateRoutingRuleRequest,
  ): Promise<RoutingRule> {
    this.guard('createRoutingRule');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/routing-rules`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<RoutingRule>;
  }

  async deleteRoutingRule(
    orgId: string,
    id: string,
  ): Promise<{ deleted: boolean }> {
    this.guard('deleteRoutingRule');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/routing-rules/${id}`,
      { method: 'DELETE' },
    ) as Promise<{ deleted: boolean }>;
  }
}

export const cloudNotificationsApi = new CloudNotificationsApi();
