import * as vscode from 'vscode';

/**
 * Validation result
 */
export interface ValidationResult {
    /** Whether the validation passed */
    valid: boolean;
    /** Error message if validation failed */
    error?: string;
}

/**
 * Validate server URL format
 * @param url Server URL to validate
 * @returns Validation result
 */
export function validateServerUrl(url: string): ValidationResult {
    if (!url || url.trim().length === 0) {
        return {
            valid: false,
            error: 'Server URL cannot be empty'
        };
    }

    try {
        const parsed = new URL(url);
        
        // Must be HTTP or HTTPS
        if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
            return {
                valid: false,
                error: 'Server URL must use http:// or https:// protocol'
            };
        }

        // Must have a hostname
        if (!parsed.hostname || parsed.hostname.length === 0) {
            return {
                valid: false,
                error: 'Server URL must include a hostname'
            };
        }

        return { valid: true };
    } catch (error) {
        return {
            valid: false,
            error: `Invalid URL format: ${error instanceof Error ? error.message : 'Unknown error'}`
        };
    }
}

/**
 * Validate timeout value
 * @param timeout Timeout in milliseconds
 * @returns Validation result
 */
export function validateTimeout(timeout: number): ValidationResult {
    if (typeof timeout !== 'number' || isNaN(timeout)) {
        return {
            valid: false,
            error: 'Timeout must be a number'
        };
    }

    if (timeout <= 0) {
        return {
            valid: false,
            error: 'Timeout must be a positive number'
        };
    }

    if (timeout > 300000) { // 5 minutes max
        return {
            valid: false,
            error: 'Timeout cannot exceed 300000ms (5 minutes)'
        };
    }

    return { valid: true };
}

/**
 * Validate retry attempts
 * @param attempts Number of retry attempts
 * @returns Validation result
 */
export function validateRetryAttempts(attempts: number): ValidationResult {
    if (typeof attempts !== 'number' || isNaN(attempts)) {
        return {
            valid: false,
            error: 'Retry attempts must be a number'
        };
    }

    if (attempts < 0) {
        return {
            valid: false,
            error: 'Retry attempts cannot be negative'
        };
    }

    if (attempts > 10) {
        return {
            valid: false,
            error: 'Retry attempts should not exceed 10'
        };
    }

    return { valid: true };
}

/**
 * Validate delay value
 * @param delay Delay in milliseconds
 * @returns Validation result
 */
export function validateDelay(delay: number): ValidationResult {
    if (typeof delay !== 'number' || isNaN(delay)) {
        return {
            valid: false,
            error: 'Delay must be a number'
        };
    }

    if (delay < 0) {
        return {
            valid: false,
            error: 'Delay cannot be negative'
        };
    }

    if (delay > 60000) { // 1 minute max
        return {
            valid: false,
            error: 'Delay should not exceed 60000ms (1 minute)'
        };
    }

    return { valid: true };
}

/**
 * Validate all MockForge configuration settings
 * @param config VS Code workspace configuration
 * @returns Validation result with details
 */
export function validateConfiguration(config: vscode.WorkspaceConfiguration): ValidationResult {
    // Validate server URL
    const serverUrl = config.get<string>('serverUrl', 'http://localhost:3000');
    const urlValidation = validateServerUrl(serverUrl);
    if (!urlValidation.valid) {
        return urlValidation;
    }

    // Validate HTTP timeout
    const httpTimeout = config.get<number>('http.timeout', 5000);
    const timeoutValidation = validateTimeout(httpTimeout);
    if (!timeoutValidation.valid) {
        return {
            valid: false,
            error: `HTTP timeout: ${timeoutValidation.error}`
        };
    }

    // Validate HTTP retry attempts
    const httpRetryAttempts = config.get<number>('http.retryAttempts', 3);
    const retryAttemptsValidation = validateRetryAttempts(httpRetryAttempts);
    if (!retryAttemptsValidation.valid) {
        return {
            valid: false,
            error: `HTTP retry attempts: ${retryAttemptsValidation.error}`
        };
    }

    // Validate HTTP retry delay
    const httpRetryDelay = config.get<number>('http.retryDelay', 1000);
    const retryDelayValidation = validateDelay(httpRetryDelay);
    if (!retryDelayValidation.valid) {
        return {
            valid: false,
            error: `HTTP retry delay: ${retryDelayValidation.error}`
        };
    }

    // Validate reconnect initial delay
    const reconnectInitialDelay = config.get<number>('reconnect.initialDelay', 1000);
    const reconnectInitialDelayValidation = validateDelay(reconnectInitialDelay);
    if (!reconnectInitialDelayValidation.valid) {
        return {
            valid: false,
            error: `Reconnect initial delay: ${reconnectInitialDelayValidation.error}`
        };
    }

    // Validate reconnect max delay
    const reconnectMaxDelay = config.get<number>('reconnect.maxDelay', 30000);
    const reconnectMaxDelayValidation = validateDelay(reconnectMaxDelay);
    if (!reconnectMaxDelayValidation.valid) {
        return {
            valid: false,
            error: `Reconnect max delay: ${reconnectMaxDelayValidation.error}`
        };
    }

    // Validate reconnect max retries
    const reconnectMaxRetries = config.get<number>('reconnect.maxRetries', 10);
    const reconnectMaxRetriesValidation = validateRetryAttempts(reconnectMaxRetries);
    if (!reconnectMaxRetriesValidation.valid) {
        return {
            valid: false,
            error: `Reconnect max retries: ${reconnectMaxRetriesValidation.error}`
        };
    }

    return { valid: true };
}

