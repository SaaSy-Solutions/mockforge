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
}

export const aiStudioApi = new AiStudioApiService();
