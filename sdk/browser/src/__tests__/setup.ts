import { vi } from 'vitest';
import { serialize, deserialize } from 'node:v8';
import 'whatwg-fetch';
import 'fake-indexeddb/auto';

/**
 * Unit-test setup file
 */

// Provide Node's structured-data cloning engine inside the test DOM context.
global.structuredClone = value => deserialize(serialize(value));

// Mock fetch globally
global.fetch = vi.fn();

// Mock AbortSignal
global.AbortSignal = {
  timeout: vi.fn((ms: number) => {
    const controller = new AbortController();
    setTimeout(() => controller.abort(), ms);
    return controller.signal;
  }),
} as any;
