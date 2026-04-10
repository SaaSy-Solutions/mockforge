/**
 * Usage tracking API — report AI token consumption to the registry.
 */
import { authenticatedFetch } from '../../utils/apiClient';
import { logger } from '@/utils/logger';

/**
 * Fire-and-forget: report AI token consumption to the cloud registry.
 * Silently swallows errors so it never blocks the caller.
 */
export async function reportAiTokenUsage(tokens: number, operation: string): Promise<void> {
  if (tokens <= 0) return;

  try {
    await authenticatedFetch('/api/v1/usage/ai-tokens', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tokens, operation }),
    });
  } catch (e) {
    // Non-critical — don't block AI operations if usage reporting fails
    logger.warn('Failed to report AI token usage', e);
  }
}
