/**
 * Jest setup file
 */

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
