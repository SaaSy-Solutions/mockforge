/**
 * AI Studio API service — cloud-mode entry points for the AI surface.
 *
 * Backed by the cloud registry server's `mockforge-registry-server::ai`
 * module. See docs/cloud/CLOUD_AI_STUDIO_DESIGN.md for the design.
 *
 * Local-mode AI features continue to use `mockai.ts`, which targets the
 * embedded admin server's `/__mockforge/api/mockai/*` endpoints. These
 * cloud endpoints are only meaningful when `isCloudMode()` is true.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

/** Usage metadata that every AI Studio response embeds. */
export interface AiUsageMeta {
  /** Which key paid for this call. UI uses this to render the BYOK badge. */
  provider: 'byok' | 'platform' | 'disabled';
  /** Tokens used by this single call (prompt + completion). */
  tokens_used: number;
  /** Updated monthly counter, for the quota meter. */
  tokens_used_this_period: number;
  /** Monthly platform-token limit. -1 means unlimited. */
  tokens_limit: number;
}

// --- chat -------------------------------------------------------------------

export interface AiChatRequest {
  /** User prompt; required. */
  prompt: string;
  /** Optional system prompt override. */
  system?: string;
  /** Optional model name override. */
  model?: string;
  /** 0.0–2.0; defaults to 0.7 server-side. */
  temperature?: number;
  /** Defaults to 1024 server-side. */
  max_tokens?: number;
}

export interface AiChatResponse extends AiUsageMeta {
  /** Generated text from the LLM. */
  content: string;
}

// --- generate-openapi -------------------------------------------------------

export interface AiGenerateOpenApiRequest {
  /** Natural-language description of the API to mock. Required. */
  description: string;
  /** Optional title for the generated spec. */
  title?: string;
  model?: string;
}

export interface AiGenerateOpenApiResponse extends AiUsageMeta {
  /** Raw text returned by the LLM. */
  content: string;
  /**
   * Best-effort parsed OpenAPI 3 document. `null` when the model response
   * wasn't valid JSON; UI should fall back to `content` in that case.
   */
  spec: unknown | null;
}

// --- explain-rule -----------------------------------------------------------

export interface AiExplainRuleRequest {
  /** Identifier for the rule (used in the prompt). Required. */
  rule_id: string;
  /** Rule definition object — passed through to the LLM as JSON context. */
  definition: unknown;
  /** Optional surrounding context (e.g. workspace name). */
  context?: string;
  model?: string;
}

export interface AiExplainRuleResponse extends AiUsageMeta {
  /** Plain-language explanation of the rule. */
  explanation: string;
}

// --- voice ------------------------------------------------------------------

export interface AiVoiceProcessRequest {
  command: string;
  model?: string;
}

export interface AiVoiceProcessResponse extends AiUsageMeta {
  intent: unknown | null;
  content: string;
}

export interface AiVoiceTranspileHookRequest {
  description: string;
  model?: string;
}

export interface AiVoiceTranspileHookResponse extends AiUsageMeta {
  hook_source: string;
  content: string;
}

export interface AiVoiceCreateScenarioRequest {
  description: string;
  workspace_context?: string;
  model?: string;
}

export interface AiVoiceCreateScenarioResponse extends AiUsageMeta {
  scenario: unknown | null;
  content: string;
}

// --- mockai (cloud) ---------------------------------------------------------

export interface AiRuleExplanation {
  id: string;
  workspace_id: string;
  rule_id: string;
  rule_type: string;
  confidence: number;
  source_examples: unknown;
  reasoning: string;
  pattern_matches: unknown;
  generated_at: string;
}

export interface AiListRuleExplanationsResponse {
  explanations: AiRuleExplanation[];
  total: number;
}

export interface AiLearnRequest {
  examples: Array<{ request: unknown; response: unknown }>;
  config?: unknown;
  model?: string;
}

export interface AiLearnResponse extends AiUsageMeta {
  success: boolean;
  rules_generated: {
    consistency_rules: number;
    schemas: number;
    state_machines: number;
    system_prompt: boolean;
  };
  explanations: Array<{
    rule_id: string;
    rule_type: string;
    confidence: number;
    reasoning: string;
  }>;
  total_explanations: number;
}

export interface AiGenerateFromTrafficRequest {
  since?: string;
  until?: string;
  path_pattern?: string;
  min_confidence?: number;
  model?: string;
}

export interface AiGenerateFromTrafficResponse extends AiUsageMeta {
  spec: unknown | null;
  content: string;
  metadata: {
    requests_analyzed: number;
    paths_inferred: number;
    generated_at: string;
    duration_ms: number;
  };
}

// --- quota snapshot ---------------------------------------------------------

export interface AiQuotaResponse {
  /** Which key would pay for the next call. */
  provider: 'byok' | 'platform' | 'disabled';
  /** Tokens used this billing period. */
  tokens_used_this_period: number;
  /** Monthly platform-token limit. -1 means unlimited. */
  tokens_limit: number;
  /**
   * True if a chat call right now would clear the quota check. False
   * means quota exhausted (Platform) or AI fully disabled (Free without
   * BYOK). Use to disable the send button / show an upgrade nudge.
   */
  call_allowed: boolean;
}

// --- service ----------------------------------------------------------------

class AiStudioApiService {
  private ensureCloud(method: string): void {
    if (!isCloudMode()) {
      throw new Error(
        `AI Studio ${method} is only available in cloud mode. Use mockai endpoints locally.`,
      );
    }
  }

  /** `POST /api/v1/ai-studio/chat` */
  async chat(request: AiChatRequest): Promise<AiChatResponse> {
    this.ensureCloud('chat');
    return fetchJsonWithErrorBody('/api/v1/ai-studio/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<AiChatResponse>;
  }

  /** `POST /api/v1/ai-studio/generate-openapi` */
  async generateOpenApi(
    request: AiGenerateOpenApiRequest,
  ): Promise<AiGenerateOpenApiResponse> {
    this.ensureCloud('generateOpenApi');
    return fetchJsonWithErrorBody('/api/v1/ai-studio/generate-openapi', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<AiGenerateOpenApiResponse>;
  }

  /** `POST /api/v1/ai-studio/explain-rule` */
  async explainRule(
    request: AiExplainRuleRequest,
  ): Promise<AiExplainRuleResponse> {
    this.ensureCloud('explainRule');
    return fetchJsonWithErrorBody('/api/v1/ai-studio/explain-rule', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<AiExplainRuleResponse>;
  }

  /** `GET /api/v1/ai-studio/quota` — read-only, no metering side effects. */
  async getQuota(): Promise<AiQuotaResponse> {
    this.ensureCloud('getQuota');
    return fetchJsonWithErrorBody(
      '/api/v1/ai-studio/quota',
    ) as Promise<AiQuotaResponse>;
  }

  // --- voice ----------------------------------------------------------------

  /** `POST /api/v1/ai-studio/voice/process` */
  async voiceProcess(request: AiVoiceProcessRequest): Promise<AiVoiceProcessResponse> {
    this.ensureCloud('voiceProcess');
    return fetchJsonWithErrorBody('/api/v1/ai-studio/voice/process', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<AiVoiceProcessResponse>;
  }

  /** `POST /api/v1/ai-studio/voice/transpile-hook` */
  async voiceTranspileHook(
    request: AiVoiceTranspileHookRequest,
  ): Promise<AiVoiceTranspileHookResponse> {
    this.ensureCloud('voiceTranspileHook');
    return fetchJsonWithErrorBody('/api/v1/ai-studio/voice/transpile-hook', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<AiVoiceTranspileHookResponse>;
  }

  /** `POST /api/v1/ai-studio/voice/create-workspace-scenario` */
  async voiceCreateWorkspaceScenario(
    request: AiVoiceCreateScenarioRequest,
  ): Promise<AiVoiceCreateScenarioResponse> {
    this.ensureCloud('voiceCreateWorkspaceScenario');
    return fetchJsonWithErrorBody('/api/v1/ai-studio/voice/create-workspace-scenario', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<AiVoiceCreateScenarioResponse>;
  }

  // --- mockai rules ---------------------------------------------------------

  /** `GET /api/v1/workspaces/{workspace_id}/mockai/rule-explanations` */
  async listRuleExplanations(
    workspaceId: string,
    filters?: { rule_type?: string; min_confidence?: number },
  ): Promise<AiListRuleExplanationsResponse> {
    this.ensureCloud('listRuleExplanations');
    const params = new URLSearchParams();
    if (filters?.rule_type) params.append('rule_type', filters.rule_type);
    if (filters?.min_confidence !== undefined) {
      params.append('min_confidence', filters.min_confidence.toString());
    }
    const qs = params.toString();
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${encodeURIComponent(workspaceId)}/mockai/rule-explanations${qs ? `?${qs}` : ''}`,
    ) as Promise<AiListRuleExplanationsResponse>;
  }

  /** `GET /api/v1/workspaces/{workspace_id}/mockai/rule-explanations/{rule_id}` */
  async getRuleExplanation(
    workspaceId: string,
    ruleId: string,
  ): Promise<{ explanation: AiRuleExplanation }> {
    this.ensureCloud('getRuleExplanation');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${encodeURIComponent(workspaceId)}/mockai/rule-explanations/${encodeURIComponent(ruleId)}`,
    ) as Promise<{ explanation: AiRuleExplanation }>;
  }

  /** `POST /api/v1/workspaces/{workspace_id}/mockai/learn` */
  async learnFromExamples(
    workspaceId: string,
    request: AiLearnRequest,
  ): Promise<AiLearnResponse> {
    this.ensureCloud('learnFromExamples');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${encodeURIComponent(workspaceId)}/mockai/learn`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      },
    ) as Promise<AiLearnResponse>;
  }

  /** `POST /api/v1/organizations/{org_id}/mockai/generate-openapi-from-traffic` */
  async generateOpenApiFromTraffic(
    orgId: string,
    request: AiGenerateFromTrafficRequest,
  ): Promise<AiGenerateFromTrafficResponse> {
    this.ensureCloud('generateOpenApiFromTraffic');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${encodeURIComponent(orgId)}/mockai/generate-openapi-from-traffic`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      },
    ) as Promise<AiGenerateFromTrafficResponse>;
  }
}

export const aiStudioApi = new AiStudioApiService();
