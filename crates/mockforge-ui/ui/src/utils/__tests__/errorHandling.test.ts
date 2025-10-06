import { describe, it, expect } from 'vitest';
import { getErrorMessage, getErrorDetails } from '../errorHandling';

describe('getErrorMessage', () => {
  it('extracts message from Error object', () => {
    const error = new Error('Test error message');
    expect(getErrorMessage(error)).toBe('Test error message');
  });

  it('returns string error directly', () => {
    expect(getErrorMessage('String error')).toBe('String error');
  });

  it('extracts message from object with message property', () => {
    const error = { message: 'Object error message' };
    expect(getErrorMessage(error)).toBe('Object error message');
  });

  it('extracts error from object with error property', () => {
    const error = { error: 'Object error value' };
    expect(getErrorMessage(error)).toBe('Object error value');
  });

  it('returns default message for unknown error type', () => {
    expect(getErrorMessage(null)).toBe('An unexpected error occurred');
    expect(getErrorMessage(undefined)).toBe('An unexpected error occurred');
    expect(getErrorMessage(123)).toBe('An unexpected error occurred');
  });

  it('returns default message for empty object', () => {
    expect(getErrorMessage({})).toBe('An unexpected error occurred');
  });

  it('handles nested error structures', () => {
    const error = { message: 'Nested error' };
    expect(getErrorMessage(error)).toBe('Nested error');
  });
});

describe('getErrorDetails', () => {
  it('identifies network errors', () => {
    const error = new TypeError('fetch failed');
    const details = getErrorDetails(error);

    expect(details.type).toBe('network');
    expect(details.message).toBe('Network error: Unable to connect to the server');
  });

  it('identifies HTTP 400 errors', () => {
    const error = new Error('HTTP error! status: 400');
    const details = getErrorDetails(error);

    expect(details.type).toBe('server');
    expect(details.statusCode).toBe(400);
    expect(details.message).toBe('Bad request: Please check your input');
  });

  it('identifies HTTP 401 errors', () => {
    const error = new Error('HTTP error! status: 401');
    const details = getErrorDetails(error);

    expect(details.type).toBe('server');
    expect(details.statusCode).toBe(401);
    expect(details.message).toBe('Unauthorized: Please log in again');
  });

  it('identifies HTTP 403 errors', () => {
    const error = new Error('HTTP error! status: 403');
    const details = getErrorDetails(error);

    expect(details.type).toBe('server');
    expect(details.statusCode).toBe(403);
    expect(details.message).toBe('Forbidden: You do not have permission to perform this action');
  });

  it('identifies HTTP 404 errors', () => {
    const error = new Error('HTTP error! status: 404');
    const details = getErrorDetails(error);

    expect(details.type).toBe('server');
    expect(details.statusCode).toBe(404);
    expect(details.message).toBe('Resource not found');
  });

  it('identifies HTTP 500 errors', () => {
    const error = new Error('HTTP error! status: 500');
    const details = getErrorDetails(error);

    expect(details.type).toBe('server');
    expect(details.statusCode).toBe(500);
    expect(details.message).toBe('Internal server error');
  });

  it('identifies validation errors', () => {
    const error = { issues: [{ message: 'Field is required' }] };
    const details = getErrorDetails(error);

    expect(details.type).toBe('validation');
    expect(details.message).toBe('Invalid data received from server');
    expect(details.details).toBe(error);
  });

  it('handles unknown error types', () => {
    const error = new Error('Random error');
    const details = getErrorDetails(error);

    expect(details.type).toBe('unknown');
    expect(details.message).toBe('Random error');
  });

  it('includes error details in response', () => {
    const error = new Error('Test error');
    const details = getErrorDetails(error);

    expect(details.details).toBe(error);
  });

  it('extracts status code from HTTP errors', () => {
    const error = new Error('HTTP error! status: 429');
    const details = getErrorDetails(error);

    expect(details.statusCode).toBe(429);
    expect(details.message).toBe('Too many requests: Please try again later');
  });

  it('handles HTTP 409 conflict errors', () => {
    const error = new Error('HTTP error! status: 409');
    const details = getErrorDetails(error);

    expect(details.statusCode).toBe(409);
    expect(details.message).toBe('Conflict: Resource already exists or cannot be modified');
  });

  it('handles HTTP 422 validation errors', () => {
    const error = new Error('HTTP error! status: 422');
    const details = getErrorDetails(error);

    expect(details.statusCode).toBe(422);
    expect(details.message).toBe('Validation error: Please check your input');
  });

  it('handles errors without status codes', () => {
    const error = new Error('HTTP error! status: ');
    const details = getErrorDetails(error);

    expect(details.statusCode).toBeUndefined();
  });
});
