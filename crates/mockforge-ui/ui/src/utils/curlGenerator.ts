import { logger } from '@/utils/logger';
import type { RouteInfo } from '../types';

export interface CurlOptions {
  baseUrl?: string;
  headers?: Record<string, string>;
  body?: string;
  followRedirects?: boolean;
  timeout?: number;
}

/**
 * Generates a cURL command from route information
 */
export function generateCurlCommand(
  route: RouteInfo,
  options: CurlOptions = {}
): string {
  const {
    baseUrl = 'http://localhost:3000',
    headers = {},
    body,
    followRedirects = true,
    timeout = 30
  } = options;

  const parts: string[] = ['curl'];

  // Add method if not GET
  if (route.method && route.method !== 'GET') {
    parts.push(`-X ${route.method}`);
  }

  // Add follow redirects
  if (followRedirects) {
    parts.push('-L');
  }

  // Add timeout
  parts.push(`--max-time ${timeout}`);

  // Add headers
  Object.entries(headers).forEach(([key, value]) => {
    parts.push(`-H "${key}: ${value}"`);
  });

  // Add body if present
  if (body && (route.method === 'POST' || route.method === 'PUT' || route.method === 'PATCH')) {
    parts.push(`-d '${body}'`);
  }

  // Add URL
  const url = route.path.startsWith('http') ? route.path : `${baseUrl}${route.path}`;
  parts.push(`"${url}"`);

  return parts.join(' \\\n  ');
}

/**
 * Copies text to clipboard
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Fallback for older browsers
    try {
      const textArea = document.createElement('textarea');
      textArea.value = text;
      textArea.style.position = 'fixed';
      textArea.style.left = '-999999px';
      textArea.style.top = '-999999px';
      document.body.appendChild(textArea);
      textArea.focus();
      textArea.select();
      const successful = document.execCommand('copy');
      document.body.removeChild(textArea);
      return successful;
    } catch (fallbackErr) {
      logger.error('Failed to copy to clipboard',fallbackErr);
      return false;
    }
  }
}
