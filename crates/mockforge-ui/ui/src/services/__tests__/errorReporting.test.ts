/**
 * @jest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { reportError, clearErrors } from '../errorReporting';
import { logger } from '../../utils/logger';

describe('errorReporting', () => {
  let loggerErrorSpy: any;

  beforeEach(() => {
    loggerErrorSpy = vi.spyOn(logger, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    loggerErrorSpy.mockRestore();
  });

  describe('reportError', () => {
    it('logs error to console in development', () => {
      const error = new Error('Test error');
      const context = { component: 'TestComponent' };

      reportError(error, context);

      expect(loggerErrorSpy).toHaveBeenCalledWith(
        '[Error Report]',
        error,
        context
      );
    });

    it('handles errors without context', () => {
      const error = new Error('Test error without context');

      reportError(error);

      expect(loggerErrorSpy).toHaveBeenCalledWith(
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

      expect(loggerErrorSpy).toHaveBeenCalledTimes(2);
    });
  });

  describe('clearErrors', () => {
    it('clears errors without throwing', () => {
      expect(() => clearErrors()).not.toThrow();
    });
  });
});
