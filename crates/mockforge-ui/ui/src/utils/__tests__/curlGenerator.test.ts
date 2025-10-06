/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { generateCurlCommand, copyToClipboard } from '../curlGenerator';

describe('generateCurlCommand', () => {
  it('generates basic GET request', () => {
    const route = { method: 'GET', path: '/api/users' };
    const curl = generateCurlCommand(route);

    expect(curl).toContain('curl');
    expect(curl).toContain('"http://localhost:3000/api/users"');
    expect(curl).toContain('-L');
    expect(curl).toContain('--max-time 30');
  });

  it('omits -X for GET requests', () => {
    const route = { method: 'GET', path: '/api/users' };
    const curl = generateCurlCommand(route);

    expect(curl).not.toContain('-X GET');
  });

  it('includes method for POST requests', () => {
    const route = { method: 'POST', path: '/api/users' };
    const curl = generateCurlCommand(route);

    expect(curl).toContain('-X POST');
  });

  it('includes method for PUT requests', () => {
    const route = { method: 'PUT', path: '/api/users/1' };
    const curl = generateCurlCommand(route);

    expect(curl).toContain('-X PUT');
  });

  it('includes method for DELETE requests', () => {
    const route = { method: 'DELETE', path: '/api/users/1' };
    const curl = generateCurlCommand(route);

    expect(curl).toContain('-X DELETE');
  });

  it('uses custom base URL when provided', () => {
    const route = { method: 'GET', path: '/api/users' };
    const curl = generateCurlCommand(route, { baseUrl: 'https://example.com' });

    expect(curl).toContain('"https://example.com/api/users"');
  });

  it('handles absolute URLs in path', () => {
    const route = { method: 'GET', path: 'https://api.example.com/users' };
    const curl = generateCurlCommand(route);

    expect(curl).toContain('"https://api.example.com/users"');
  });

  it('includes headers when provided', () => {
    const route = { method: 'GET', path: '/api/users' };
    const headers = {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer token123',
    };
    const curl = generateCurlCommand(route, { headers });

    expect(curl).toContain('-H "Content-Type: application/json"');
    expect(curl).toContain('-H "Authorization: Bearer token123"');
  });

  it('includes body for POST requests', () => {
    const route = { method: 'POST', path: '/api/users' };
    const body = '{"name": "John"}';
    const curl = generateCurlCommand(route, { body });

    expect(curl).toContain(`-d '${body}'`);
  });

  it('includes body for PUT requests', () => {
    const route = { method: 'PUT', path: '/api/users/1' };
    const body = '{"name": "Jane"}';
    const curl = generateCurlCommand(route, { body });

    expect(curl).toContain(`-d '${body}'`);
  });

  it('includes body for PATCH requests', () => {
    const route = { method: 'PATCH', path: '/api/users/1' };
    const body = '{"name": "Bob"}';
    const curl = generateCurlCommand(route, { body });

    expect(curl).toContain(`-d '${body}'`);
  });

  it('does not include body for GET requests', () => {
    const route = { method: 'GET', path: '/api/users' };
    const body = '{"name": "John"}';
    const curl = generateCurlCommand(route, { body });

    expect(curl).not.toContain('-d');
  });

  it('does not include body for DELETE requests', () => {
    const route = { method: 'DELETE', path: '/api/users/1' };
    const body = '{"confirm": true}';
    const curl = generateCurlCommand(route, { body });

    expect(curl).not.toContain('-d');
  });

  it('disables follow redirects when specified', () => {
    const route = { method: 'GET', path: '/api/users' };
    const curl = generateCurlCommand(route, { followRedirects: false });

    expect(curl).not.toContain('-L');
  });

  it('uses custom timeout when provided', () => {
    const route = { method: 'GET', path: '/api/users' };
    const curl = generateCurlCommand(route, { timeout: 60 });

    expect(curl).toContain('--max-time 60');
  });

  it('formats command with line breaks', () => {
    const route = { method: 'POST', path: '/api/users' };
    const headers = { 'Content-Type': 'application/json' };
    const body = '{"name": "John"}';
    const curl = generateCurlCommand(route, { headers, body });

    expect(curl).toContain('\\\n');
  });

  it('generates complete command with all options', () => {
    const route = { method: 'POST', path: '/api/users' };
    const options = {
      baseUrl: 'https://api.example.com',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': 'Bearer token123',
      },
      body: '{"name": "John", "email": "john@example.com"}',
      followRedirects: true,
      timeout: 45,
    };
    const curl = generateCurlCommand(route, options);

    expect(curl).toContain('curl');
    expect(curl).toContain('-X POST');
    expect(curl).toContain('-L');
    expect(curl).toContain('--max-time 45');
    expect(curl).toContain('-H "Content-Type: application/json"');
    expect(curl).toContain('-H "Authorization: Bearer token123"');
    expect(curl).toContain(`-d '${options.body}'`);
    expect(curl).toContain('"https://api.example.com/api/users"');
  });
});

describe('copyToClipboard', () => {
  beforeEach(() => {
    // Mock clipboard API
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn(),
      },
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('copies text using modern clipboard API', async () => {
    const mockWriteText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: {
        writeText: mockWriteText,
      },
    });

    const text = 'curl -X GET "https://example.com"';
    const result = await copyToClipboard(text);

    expect(result).toBe(true);
    expect(mockWriteText).toHaveBeenCalledWith(text);
  });

  it('returns true on successful copy', async () => {
    const mockWriteText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: {
        writeText: mockWriteText,
      },
    });

    const result = await copyToClipboard('test text');
    expect(result).toBe(true);
  });

  it('falls back to execCommand when clipboard API fails', async () => {
    const mockWriteText = vi.fn().mockRejectedValue(new Error('Not allowed'));
    Object.assign(navigator, {
      clipboard: {
        writeText: mockWriteText,
      },
    });

    // Mock document.execCommand
    document.execCommand = vi.fn().mockReturnValue(true);

    const result = await copyToClipboard('test text');
    expect(result).toBe(true);
  });

  it('returns false when all copy methods fail', async () => {
    const mockWriteText = vi.fn().mockRejectedValue(new Error('Not allowed'));
    Object.assign(navigator, {
      clipboard: {
        writeText: mockWriteText,
      },
    });

    // Mock document.execCommand to fail
    document.execCommand = vi.fn().mockReturnValue(false);

    const result = await copyToClipboard('test text');
    expect(result).toBe(false);
  });
});
