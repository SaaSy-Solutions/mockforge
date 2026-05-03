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

export interface AiChatRequest {
  /** User prompt; required. */
  prompt: string;
  /** Optional system prompt override. */
  system?: string;
  /** Optional model name override (e.g. `gpt-4o-mini`, `claude-3-5-sonnet-20241022`). */
  model?: string;
  /** 0.0–2.0; defaults to 0.7 server-side. */
  temperature?: number;
  /** Defaults to 1024 server-side. */
  max_tokens?: number;
}

export interface AiChatResponse {
  /** Generated text from the LLM. */
  content: string;
  /** Which key paid for this call. UI uses this to render the BYOK badge. */
  provider: 'byok' | 'platform' | 'disabled';
  /** Tokens used by this single call (prompt + completion). */
  tokens_used: number;
  /** Updated monthly counter, for the quota meter. */
  tokens_used_this_period: number;
  /** Monthly platform-token limit. -1 means unlimited. */
  tokens_limit: number;
}

class AiStudioApiService {
  /**
   * Send a chat completion request through the cloud AI proxy.
   * Throws if invoked outside cloud mode — local mode has no equivalent route.
   */
  async chat(request: AiChatRequest): Promise<AiChatResponse> {
    if (!isCloudMode()) {
      throw new Error(
        'AI Studio chat is only available in cloud mode. Use mockai endpoints locally.',
      );
    }
    return fetchJsonWithErrorBody('/api/v1/ai-studio/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<AiChatResponse>;
  }
}

export const aiStudioApi = new AiStudioApiService();
