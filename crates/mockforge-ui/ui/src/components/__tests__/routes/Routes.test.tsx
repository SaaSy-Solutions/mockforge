/**
 * @jest-environment jsdom
 */

import { describe, it, expect } from 'vitest';
import { RequestLogSchema } from '../../../schemas/api';

describe('Routes Components', () => {
  it('validates request log payload shape', () => {
    const parsed = RequestLogSchema.parse({
      id: 'req-1',
      timestamp: new Date().toISOString(),
      method: 'GET',
      path: '/api/users',
      status_code: 200,
      response_time_ms: 42,
      response_size_bytes: 100,
      request_size_bytes: 0,
    });

    expect(parsed.path).toBe('/api/users');
    expect(parsed.status_code).toBe(200);
  });
});
