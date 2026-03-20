/**
 * Shared API client utilities used by all domain-specific API services.
 */
import { authenticatedFetch } from '../../utils/apiClient';
import { safeValidateApiResponse } from '../../schemas/api';
import { logger } from '@/utils/logger';

export { authenticatedFetch };

/**
 * Standard fetchJson: makes an authenticated request and unwraps .data if present.
 */
export async function fetchJson(url: string, options?: RequestInit): Promise<unknown> {
  const response = await authenticatedFetch(url, options);
  if (!response.ok) {
    if (response.status === 401) {
      throw new Error('Authentication required');
    }
    if (response.status === 403) {
      throw new Error('Access denied');
    }
    throw new Error(`HTTP error! status: ${response.status}`);
  }
  const json = await response.json();
  return json.data || json;
}

/**
 * fetchJson variant that also parses error body for a richer error message.
 */
export async function fetchJsonWithErrorBody(url: string, options?: RequestInit): Promise<unknown> {
  const response = await authenticatedFetch(url, options);
  if (!response.ok) {
    if (response.status === 401) {
      throw new Error('Authentication required');
    }
    if (response.status === 403) {
      throw new Error('Access denied');
    }
    const error = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(error.error || `HTTP error! status: ${response.status}`);
  }
  const json = await response.json();
  return json.data || json;
}

/**
 * fetchJson variant that parses error text (trying JSON then falling back).
 */
export async function fetchJsonWithErrorText(url: string, options?: RequestInit): Promise<unknown> {
  const response = await authenticatedFetch(url, options);
  if (!response.ok) {
    if (response.status === 401) {
      throw new Error('Authentication required');
    }
    if (response.status === 403) {
      throw new Error('Access denied');
    }
    const errorText = await response.text();
    let errorMessage = `HTTP error! status: ${response.status}`;
    try {
      const errorJson = JSON.parse(errorText);
      errorMessage = errorJson.error || errorMessage;
    } catch {
      // Not JSON, use default message
    }
    throw new Error(errorMessage);
  }
  const json = await response.json();
  return json.data || json;
}

/**
 * fetchJson + Zod schema validation.
 */
export async function fetchJsonWithValidation<T>(
  url: string,
  schema: Parameters<typeof safeValidateApiResponse>[0],
  options?: RequestInit
): Promise<T> {
  const data = await fetchJson(url, options);
  const result = safeValidateApiResponse(schema, data);

  if (!result.success) {
    if (import.meta.env.DEV) {
      logger.error('API validation error', result.error.format());
    }
    throw new Error(`API response validation failed: ${result.error.message}`);
  }

  return result.data as T;
}

/**
 * Unauthenticated fetchJson (used by ProxyApiService).
 */
export async function fetchJsonUnauthenticated(url: string, options?: RequestInit): Promise<unknown> {
  const response = await fetch(url, options);
  if (!response.ok) {
    const errorText = await response.text();
    let errorMessage = `HTTP error! status: ${response.status}`;
    try {
      const errorJson = JSON.parse(errorText);
      errorMessage = errorJson.error || errorMessage;
    } catch {
      // Not JSON, use default message
    }
    throw new Error(errorMessage);
  }
  const json = await response.json();
  return json.data || json;
}

/**
 * fetchJson variant that parses error body using .message field.
 */
export async function fetchJsonWithErrorMessage(url: string, options?: RequestInit): Promise<unknown> {
  const response = await authenticatedFetch(url, options);
  if (!response.ok) {
    if (response.status === 401) {
      throw new Error('Authentication required');
    }
    if (response.status === 403) {
      throw new Error('Access denied');
    }
    const errorData = await response.json().catch(() => ({}));
    throw new Error(errorData.message || `HTTP error! status: ${response.status}`);
  }
  const json = await response.json();
  return json.data || json;
}
