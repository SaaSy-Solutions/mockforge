/**
 * @jest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { reportError, clearErrors } from '../errorReporting';

describe('errorReporting', () => {
  let consoleErrorSpy: any;

  beforeEach(() => {
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  describe('reportError', () => {
    it('logs error to console in development', () => {
      const error = new Error('Test error');
      const context = { component: 'TestComponent' };

      reportError(error, context);

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        '[Error Report]',
        error,
        context
      );
    });

    it('handles errors without context', () => {
      const error = new Error('Test error without context');

      reportError(error);

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        '[Error Report]',
        error,
        undefined
      );
    });

    it('handles different error types', () => {
      const typeError = new TypeError('Type error');
      const rangeError = new RangeError('Range error');

      reportError(typeError);
      reportError(rangeError);

      expect(consoleErrorSpy).toHaveBeenCalledTimes(2);
    });
  });

  describe('clearErrors', () => {
    it('clears errors without throwing', () => {
      expect(() => clearErrors()).not.toThrow();
    });
  });
});
