import { logger } from '@/utils/logger';
/**
 * Error handling utilities for consistent error processing across the app
 */

export interface ErrorDetails {
  message: string;
  type: 'network' | 'validation' | 'server' | 'unknown';
  statusCode?: number;
  details?: unknown;
}

/**
 * Extract a user-friendly error message from various error types
 */
export function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === 'string') {
    return error;
  }

  if (error && typeof error === 'object') {
    if ('message' in error && typeof error.message === 'string') {
      return error.message;
    }
    if ('error' in error && typeof error.error === 'string') {
      return error.error;
    }
  }

  return 'An unexpected error occurred';
}

/**
 * Extract detailed error information for logging and debugging
 */
export function getErrorDetails(error: unknown): ErrorDetails {
  // Network/Fetch errors
  if (error instanceof TypeError && error.message.includes('fetch')) {
    return {
      message: 'Network error: Unable to connect to the server',
      type: 'network',
    };
  }

  // HTTP errors
  if (error instanceof Error && error.message.startsWith('HTTP error! status:')) {
    const statusMatch = error.message.match(/status: (\d+)/);
    const statusCode = statusMatch ? parseInt(statusMatch[1], 10) : undefined;

    return {
      message: getHttpErrorMessage(statusCode),
      type: 'server',
      statusCode,
      details: error,
    };
  }

  // Validation errors (Zod)
  if (error && typeof error === 'object' && 'issues' in error) {
    return {
      message: 'Invalid data received from server',
      type: 'validation',
      details: error,
    };
  }

  // Generic error
  return {
    message: getErrorMessage(error),
    type: 'unknown',
    details: error,
  };
}

/**
 * Get user-friendly HTTP error messages
 */
function getHttpErrorMessage(statusCode?: number): string {
  if (!statusCode) return 'Server error occurred';

  switch (statusCode) {
    case 400:
      return 'Bad request: Please check your input';
    case 401:
      return 'Unauthorized: Please log in again';
    case 403:
      return 'Forbidden: You do not have permission to perform this action';
    case 404:
      return 'Resource not found';
    case 409:
      return 'Conflict: Resource already exists or cannot be modified';
    case 422:
      return 'Validation error: Please check your input';
    case 429:
      return 'Too many requests: Please try again later';
    case 500:
      return 'Internal server error';
    case 502:
      return 'Bad gateway: Server is temporarily unavailable';
    case 503:
      return 'Service unavailable: Please try again later';
    case 504:
      return 'Gateway timeout: Server took too long to respond';
    default:
      if (statusCode >= 400 && statusCode < 500) {
        return 'Client error occurred';
      } else if (statusCode >= 500) {
        return 'Server error occurred';
      }
      return 'An error occurred';
  }
}

/**
 * Log error details in development mode
 */
export function logError(error: unknown, context?: string): void {
  if (import.meta.env.DEV) {
    const details = getErrorDetails(error);
    logger.error(
      `[Error${context ? ` - ${context}` : ''}]`,
      details.message,
      '\nType:',
      details.type,
      '\nDetails:',
      details.details || error
    );
  }
}

/**
 * Create a user-friendly error handler for async operations
 */
export function handleAsyncError(
  error: unknown,
  context: string,
  onError?: (message: string) => void
): void {
  const details = getErrorDetails(error);
  logError(error, context);

  if (onError) {
    onError(details.message);
  }
}

/**
 * Sanitize input to prevent XSS and other injection attacks
 */
export function sanitizeInput(input: string): string {
  // Remove potential HTML/script tags
  return input
    .replace(/[<>]/g, '')
    .replace(/javascript:/gi, '')
    .replace(/on\w+=/gi, '')
    .trim();
}

/**
 * Validate file upload before processing
 */
export interface FileValidationOptions {
  maxSize?: number; // in bytes
  allowedTypes?: string[];
  allowedExtensions?: string[];
}

export function validateFile(
  file: File,
  options: FileValidationOptions = {}
): { valid: boolean; error?: string } {
  const {
    maxSize = 10 * 1024 * 1024, // 10MB default
    allowedTypes = [],
    allowedExtensions = [],
  } = options;

  // Check file size
  if (file.size > maxSize) {
    return {
      valid: false,
      error: `File size exceeds maximum allowed size of ${(maxSize / (1024 * 1024)).toFixed(1)}MB`,
    };
  }

  // Check file type
  if (allowedTypes.length > 0 && !allowedTypes.includes(file.type)) {
    return {
      valid: false,
      error: `File type "${file.type}" is not allowed`,
    };
  }

  // Check file extension
  if (allowedExtensions.length > 0) {
    const extension = file.name.split('.').pop()?.toLowerCase();
    if (!extension || !allowedExtensions.includes(extension)) {
      return {
        valid: false,
        error: `File extension "${extension}" is not allowed`,
      };
    }
  }

  return { valid: true };
}
