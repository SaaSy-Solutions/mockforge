import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // Global setup to start MockForge server
    globalSetup: './tests/setup.ts',

    // Test environment
    environment: 'node',

    // Test timeout
    testTimeout: 10000,

    // Reporters
    reporters: ['verbose'],

    // Coverage settings
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
    },
  },
});
