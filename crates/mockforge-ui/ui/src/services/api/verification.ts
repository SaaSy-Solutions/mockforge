/**
 * Verification API service — verify, count, sequence, never, at-least.
 */
import type { VerificationRequest, VerificationCount, VerificationResult } from '../../types';
import { fetchJsonWithErrorMessage } from './client';

class VerificationApiService {
  async verify(pattern: VerificationRequest, expected: VerificationCount): Promise<VerificationResult> {
    return fetchJsonWithErrorMessage('/__mockforge/verification/verify', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pattern, expected }),
    }) as Promise<VerificationResult>;
  }

  async count(pattern: VerificationRequest): Promise<{ count: number }> {
    return fetchJsonWithErrorMessage('/__mockforge/verification/count', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pattern }),
    }) as Promise<{ count: number }>;
  }

  async verifySequence(patterns: VerificationRequest[]): Promise<VerificationResult> {
    return fetchJsonWithErrorMessage('/__mockforge/verification/sequence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ patterns }),
    }) as Promise<VerificationResult>;
  }

  async verifyNever(pattern: VerificationRequest): Promise<VerificationResult> {
    return fetchJsonWithErrorMessage('/__mockforge/verification/never', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(pattern),
    }) as Promise<VerificationResult>;
  }

  async verifyAtLeast(pattern: VerificationRequest, min: number): Promise<VerificationResult> {
    return fetchJsonWithErrorMessage('/__mockforge/verification/at-least', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pattern, min }),
    }) as Promise<VerificationResult>;
  }
}

export { VerificationApiService };
