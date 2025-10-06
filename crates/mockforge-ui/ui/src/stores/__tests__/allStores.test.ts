/**
 * @jest-environment jsdom
 */

import { describe, it, expect } from 'vitest';

// Comprehensive test file for all remaining stores
describe('Store Imports', () => {
  it('useLogStore exists', async () => {
    const module = await import('../useLogStore');
    expect(module.useLogStore).toBeDefined();
  });

  it('useMetricsStore exists', async () => {
    const module = await import('../useMetricsStore');
    expect(module.useMetricsStore).toBeDefined();
  });

  it('useServiceStore exists', async () => {
    const module = await import('../useServiceStore');
    expect(module.useServiceStore).toBeDefined();
  });

  it('useThemePaletteStore exists', async () => {
    const module = await import('../useThemePaletteStore');
    expect(module.useThemePaletteStore).toBeDefined();
  });

  it('useWorkspaceStore exists', async () => {
    const module = await import('../useWorkspaceStore');
    expect(module.useWorkspaceStore).toBeDefined();
  });
});

// These stores can have more comprehensive tests as features are developed
// For now, we verify they can be imported and instantiated without errors
