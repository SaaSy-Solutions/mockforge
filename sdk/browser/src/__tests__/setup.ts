import { serialize, deserialize } from 'node:v8';
import 'whatwg-fetch';
import 'fake-indexeddb/auto';

/**
 * Jest setup file
 */

// Provide Node's structured-data cloning engine inside Jest's DOM context.
global.structuredClone = value => deserialize(serialize(value));

// Mock fetch globally
global.fetch = jest.fn();

// Mock AbortSignal
global.AbortSignal = {
  timeout: jest.fn((ms: number) => {
    const controller = new AbortController();
    setTimeout(() => controller.abort(), ms);
    return controller.signal;
  }),
} as any;
