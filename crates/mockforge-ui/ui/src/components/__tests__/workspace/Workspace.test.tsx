/**
 * @jest-environment jsdom
 */

import { describe, it, expect } from 'vitest';
import { WorkspaceSummarySchema } from '../../../schemas/api';

describe('Workspace Components', () => {
  it('applies default is_active=false when omitted', () => {
    const parsed = WorkspaceSummarySchema.parse({
      id: 'ws-1',
      name: 'Workspace One',
    });

    expect(parsed.is_active).toBe(false);
  });

  it('preserves explicit is_active=true', () => {
    const parsed = WorkspaceSummarySchema.parse({
      id: 'ws-2',
      name: 'Workspace Two',
      is_active: true,
    });

    expect(parsed.is_active).toBe(true);
  });
});
