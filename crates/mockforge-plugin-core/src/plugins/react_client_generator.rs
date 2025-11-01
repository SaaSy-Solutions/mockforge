//! React Client Generator Plugin
//!
//! Generates React hooks and TypeScript types from OpenAPI specifications
//! for easy integration with React applications.

use crate::client_generator::{
    ClientGenerationResult, ClientGeneratorConfig, ClientGeneratorPlugin, GeneratedFile,
    GenerationMetadata, OpenApiSpec,
};
use crate::types::{PluginError, PluginMetadata, Result};
use handlebars::Handlebars;
use serde_json::{json, Value};
use std::collections::HashMap;

/// React client generator plugin
pub struct ReactClientGenerator {
    /// Template registry for code generation
    templates: Handlebars<'static>,
}

impl ReactClientGenerator {
    /// Create a new React client generator
    pub fn new() -> Result<Self> {
        let mut templates = Handlebars::new();

        // Register templates for React code generation
        Self::register_templates(&mut templates)?;

        Ok(Self { templates })
    }

    /// Process a schema JSON value to add required flags to properties
    /// This makes it easier for Handlebars templates to check required fields
    fn process_schema_with_required_flags(mut schema: Value) -> Value {
        // First, extract and clone required fields list (to avoid borrow conflicts)
        let required_fields: Vec<String> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_default();

        // Then, modify properties (mutable borrow)
        if let Some(properties) = schema.get_mut("properties").and_then(|p| p.as_object_mut()) {
            for (prop_name, prop_value) in properties.iter_mut() {
                // Add required flag to each property
                if let Some(prop_obj) = prop_value.as_object_mut() {
                    prop_obj.insert(
                        "required".to_string(),
                        serde_json::Value::Bool(required_fields.contains(prop_name)),
                    );
                }
            }
        }
        schema
    }

    /// Register Handlebars templates for React code generation
    fn register_templates(templates: &mut Handlebars<'static>) -> Result<()> {
        // TypeScript types template
        templates
            .register_template_string(
                "types",
                r#"// Generated TypeScript types for {{api_title}}
// API Version: {{api_version}}

{{#each schemas}}
export interface {{@key}} {
{{#each this.properties}}
  {{#if (lookup ../this.required @key)}}
  {{@key}}: {{> typescript_type this}};
  {{else}}
  {{@key}}?: {{> typescript_type this}};
  {{/if}}
{{/each}}
}

{{/each}}

// API Response types
{{#each operations}}
export interface {{response_type_name}} {
{{#each responses}}
{{#if (eq @key "200")}}
{{#if this.content}}
{{#each this.content}}
{{#if (eq @key "application/json")}}
{{#if this.schema}}
{{#if this.schema.properties}}
{{#each this.schema.properties}}
  {{@key}}{{#unless this.required}}?{{/unless}}: {{> typescript_type this}};
{{/each}}
{{else}}
{{#if (eq this.schema.type "object")}}
  [key: string]: any;
{{else}}
{{#if this.schema.type}}
  value: {{> typescript_type this.schema}};
{{/if}}
{{/if}}
{{/if}}
{{/if}}
{{/if}}
{{/each}}
{{/if}}
{{/if}}
{{/each}}
}

{{/each}}

// API Request types
{{#each operations}}
{{#if request_body}}
export interface {{request_type_name}} {
{{#each request_body.content}}
{{#if (eq @key "application/json")}}
{{#if this.schema}}
{{#if this.schema.properties}}
{{#each this.schema.properties}}
  {{@key}}{{#unless this.required}}?{{/unless}}: {{> typescript_type this}};
{{/each}}
{{else}}
{{#if (eq this.schema.type "object")}}
  [key: string]: any;
{{else}}
{{#if this.schema.type}}
  value: {{> typescript_type this.schema}};
{{/if}}
{{/if}}
{{/if}}
{{/if}}
{{/if}}
{{/each}}
}

{{/if}}
{{/each}}"#,
            )
            .map_err(|e| {
                PluginError::execution(format!("Failed to register types template: {}", e))
            })?;

        // React hooks template
        templates.register_template_string(
            "hooks",
            r#"// Generated React hooks for {{api_title}}
// API Version: {{api_version}}

import { useState, useEffect, useCallback } from 'react';

// ============================================================================
// Error Handling
// ============================================================================

/**
 * Base API Error class with structured error information
 */
export class ApiError extends Error {
  constructor(
    public status: number,
    public statusText: string,
    public body?: any,
    message?: string
  ) {
    super(message || `API Error: ${status} ${statusText}`);
    this.name = 'ApiError';
    Object.setPrototypeOf(this, ApiError.prototype);
  }

  /**
   * Check if error is a client error (4xx)
   */
  isClientError(): boolean {
    return this.status >= 400 && this.status < 500;
  }

  /**
   * Check if error is a server error (5xx)
   */
  isServerError(): boolean {
    return this.status >= 500;
  }

  /**
   * Get error details from response body if available
   */
  getErrorDetails(): any {
    return this.body;
  }

  /**
   * Get verbose error message with validation details
   */
  getVerboseMessage(): string {
    let message = `${this.status} ${this.statusText}`;

    if (this.body) {
      if (typeof this.body === 'object') {
        // Handle validation errors
        if (this.body.errors && Array.isArray(this.body.errors)) {
          const validationErrors = this.body.errors
            .map((err: any) => {
              if (typeof err === 'string') return err;
              if (err.field && err.message) return `${err.field}: ${err.message}`;
              if (err.message) return err.message;
              return JSON.stringify(err);
            })
            .join('; ');
          message += ` - Validation errors: ${validationErrors}`;
        } else if (this.body.message) {
          message += ` - ${this.body.message}`;
        } else if (this.body.error) {
          message += ` - ${this.body.error}`;
          if (this.body.error_description) {
            message += ` (${this.body.error_description})`;
          }
        } else {
          message += ` - ${JSON.stringify(this.body)}`;
        }
      } else if (typeof this.body === 'string') {
        message += ` - ${this.body}`;
      }
    }

    return message;
  }
}

/**
 * Error thrown when a required parameter is missing
 */
export class RequiredError extends Error {
  constructor(public field: string, message?: string) {
    super(message || `Required parameter '${field}' was null or undefined`);
    this.name = 'RequiredError';
    Object.setPrototypeOf(this, RequiredError.prototype);
  }
}

// ============================================================================
// Configuration
// ============================================================================

/**
 * OAuth2 Flow Configuration
 *
 * ⚠️ SECURITY WARNING:
 * - NEVER include clientSecret in browser/client-side code
 * - Client secrets should only be used in server-side applications
 * - For browser apps, use authorization_code flow with PKCE (recommended)
 * - Tokens stored in localStorage are vulnerable to XSS attacks
 * - Consider using httpOnly cookies or secure storage for production
 */
export interface OAuth2Config {
  /** OAuth2 client ID */
  clientId: string;
  /**
   * OAuth2 client secret (for client_credentials flow)
   * ⚠️ SECURITY: Only use in server-side apps. NEVER expose in browser code!
   * For browser apps, use authorization_code flow without client secret
   */
  clientSecret?: string;
  /** Authorization URL (for authorization_code flow) */
  authorizationUrl?: string;
  /** Token URL for obtaining access tokens */
  tokenUrl: string;
  /** Redirect URI (for authorization_code flow) */
  redirectUri?: string;
  /** Scopes to request */
  scopes?: string[];
  /** OAuth2 flow type */
  flow?: 'authorization_code' | 'client_credentials' | 'implicit' | 'password';
  /** Token storage key (default: 'oauth2_token') */
  tokenStorageKey?: string;
  /** Callback for token refresh */
  onTokenRefresh?: (token: string) => void | Promise<void>;
  /** State parameter for CSRF protection (auto-generated if not provided) */
  state?: string;
  /** PKCE code verifier for authorization_code flow (recommended for browser apps) */
  codeVerifier?: string;
}

/**
 * API Configuration with support for authentication and interceptors
 */
export interface ApiConfig {
  /** Base URL for API requests */
  baseUrl: string;
  /** Default headers to include with every request */
  headers?: Record<string, string>;
  /** Bearer token for authentication */
  accessToken?: string | (() => string | Promise<string>);
  /** API key for authentication (supports function for dynamic keys) */
  apiKey?: string | ((name: string) => string | Promise<string>);
  /** Username for basic authentication */
  username?: string;
  /** Password for basic authentication */
  password?: string;
  /** OAuth2 configuration for OAuth flows */
  oauth2?: OAuth2Config;
  /** Request interceptor - called before each request */
  requestInterceptor?: (request: RequestInit) => RequestInit | Promise<RequestInit>;
  /** Response interceptor - called after each response */
  responseInterceptor?: <T>(response: Response, data: T) => T | Promise<T>;
  /** Error interceptor - called when a request fails */
  errorInterceptor?: (error: ApiError) => ApiError | Promise<ApiError>;
  /** Timeout in milliseconds (default: 30000) */
  timeout?: number;
  /** Enable request/response validation (default: false) */
  validateRequests?: boolean;
  /** Enable verbose error messages (default: false) */
  verboseErrors?: boolean;
}

/**
 * OAuth2 Token Manager
 * Handles OAuth2 flows and token refresh
 */
class OAuth2TokenManager {
  constructor(private config: OAuth2Config) {}

  /**
   * Get stored access token with expiration check
   * ⚠️ SECURITY: Tokens in localStorage are vulnerable to XSS attacks
   */
  private getStoredToken(): { token: string; expiresAt?: number } | null {
    const key = this.config.tokenStorageKey || 'oauth2_token';
    if (typeof localStorage !== 'undefined') {
      const stored = localStorage.getItem(key);
      if (!stored) return null;

      try {
        // Try to parse as JSON (with expiration) or use as plain string
        const parsed = JSON.parse(stored);
        if (parsed.token && parsed.expiresAt) {
          // Check if token is expired
          if (Date.now() >= parsed.expiresAt * 1000) {
            localStorage.removeItem(key);
            return null;
          }
          return parsed;
        }
        // Legacy format (plain string) - return as token
        return { token: parsed };
      } catch {
        // Plain string format
        return { token: stored };
      }
    }
    return null;
  }

  /**
   * Store access token with optional expiration
   * ⚠️ SECURITY: Tokens stored in localStorage are vulnerable to XSS attacks
   * Consider using httpOnly cookies or secure storage for production apps
   */
  private async storeToken(token: string, expiresIn?: number): Promise<void> {
    const key = this.config.tokenStorageKey || 'oauth2_token';
    if (typeof localStorage !== 'undefined') {
      // Store token with expiration if provided
      const tokenData = expiresIn
        ? JSON.stringify({ token, expiresAt: Math.floor(Date.now() / 1000) + expiresIn })
        : token;
      localStorage.setItem(key, tokenData);
    }
    if (this.config.onTokenRefresh) {
      await this.config.onTokenRefresh(token);
    }
  }

  /**
   * Get access token via client_credentials flow
   * ⚠️ SECURITY WARNING: This flow requires a client secret which should NEVER be in browser code!
   * Only use this flow in server-side applications. For browser apps, use authorization_code flow.
   */
  async getClientCredentialsToken(): Promise<string> {
    if (!this.config.clientSecret) {
      if (typeof window !== 'undefined') {
        console.warn('⚠️ SECURITY WARNING: client_credentials flow with client secret in browser code is insecure. Use authorization_code flow instead.');
      }
      throw new Error('Client secret required for client_credentials flow. ⚠️ SECURITY: Never expose client secrets in browser code!');
    }

    const params = new URLSearchParams({
      grant_type: 'client_credentials',
      client_id: this.config.clientId,
      client_secret: this.config.clientSecret,
      ...(this.config.scopes && this.config.scopes.length > 0
        ? { scope: this.config.scopes.join(' ') }
        : {}),
    });

    const response = await fetch(this.config.tokenUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: params.toString(),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Token request failed' }));
      throw new ApiError(
        response.status,
        response.statusText,
        error,
        `OAuth2 token request failed: ${error.error || response.statusText}`
      );
    }

    const data = await response.json();
    const token = data.access_token;
    if (!token) {
      throw new Error('No access_token in OAuth2 response');
    }

    // Store token with expiration if provided (expires_in is in seconds)
    await this.storeToken(token, data.expires_in);
    return token;
  }

  /**
   * Get access token via password flow
   */
  async getPasswordToken(username: string, password: string): Promise<string> {
    const params = new URLSearchParams({
      grant_type: 'password',
      username,
      password,
      client_id: this.config.clientId,
      ...(this.config.scopes && this.config.scopes.length > 0
        ? { scope: this.config.scopes.join(' ') }
        : {}),
    });

    if (this.config.clientSecret) {
      params.append('client_secret', this.config.clientSecret);
    }

    const response = await fetch(this.config.tokenUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: params.toString(),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Token request failed' }));
      throw new ApiError(
        response.status,
        response.statusText,
        error,
        `OAuth2 token request failed: ${error.error || response.statusText}`
      );
    }

    const data = await response.json();
    const token = data.access_token;
    if (!token) {
      throw new Error('No access_token in OAuth2 response');
    }

    // Store token with expiration if provided (expires_in is in seconds)
    await this.storeToken(token, data.expires_in);
    return token;
  }

  /**
   * Refresh access token using refresh_token
   */
  async refreshToken(refreshToken: string): Promise<string> {
    const params = new URLSearchParams({
      grant_type: 'refresh_token',
      refresh_token: refreshToken,
      client_id: this.config.clientId,
    });

    if (this.config.clientSecret) {
      params.append('client_secret', this.config.clientSecret);
    }

    const response = await fetch(this.config.tokenUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: params.toString(),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Token refresh failed' }));
      throw new ApiError(
        response.status,
        response.statusText,
        error,
        `OAuth2 token refresh failed: ${error.error || response.statusText}`
      );
    }

    const data = await response.json();
    const token = data.access_token;
    if (!token) {
      throw new Error('No access_token in OAuth2 refresh response');
    }

    await this.storeToken(token);
    return token;
  }

  /**
   * Get current access token (from storage or fetch new)
   * Checks token expiration before returning stored token
   */
  async getAccessToken(): Promise<string | null> {
    // Try to get stored token first (with expiration check)
    const stored = this.getStoredToken();
    if (stored && stored.token) {
      return stored.token;
    }

    // If no stored token and client_credentials flow, fetch new token
    if (this.config.flow === 'client_credentials') {
      return await this.getClientCredentialsToken();
    }

    return null;
  }

  /**
   * Initiate authorization_code flow (redirects to authorization URL)
   * Generates state parameter for CSRF protection if not provided
   * Supports PKCE if codeVerifier is provided
   */
  async authorize(): Promise<void> {
    if (!this.config.authorizationUrl || !this.config.redirectUri) {
      throw new Error('authorizationUrl and redirectUri required for authorization_code flow');
    }

    // Generate state for CSRF protection if not provided
    const state = this.config.state || this.generateRandomString(32);
    if (!this.config.state && typeof localStorage !== 'undefined') {
      localStorage.setItem(`${this.config.tokenStorageKey || 'oauth2_token'}_state`, state);
    }

    // Generate PKCE code challenge if code verifier is provided
    let codeChallenge: string | undefined;
    let codeChallengeMethod: string | undefined;
    if (this.config.codeVerifier) {
      // Use proper PKCE with SHA256 hash (RFC 7636)
      if (typeof crypto !== 'undefined' && crypto.subtle) {
        codeChallenge = await this.generateCodeChallenge(this.config.codeVerifier);
        codeChallengeMethod = 'S256';
      } else {
        // Fallback for environments without Web Crypto API
        // Note: This is less secure but allows basic PKCE functionality
        codeChallenge = this.base64UrlEncode(this.config.codeVerifier);
        codeChallengeMethod = 'plain';
      }
    }

    const params = new URLSearchParams({
      response_type: 'code',
      client_id: this.config.clientId,
      redirect_uri: this.config.redirectUri,
      state,
      ...(this.config.scopes && this.config.scopes.length > 0
        ? { scope: this.config.scopes.join(' ') }
        : {}),
      ...(codeChallenge ? { code_challenge: codeChallenge, code_challenge_method: codeChallengeMethod! } : {}),
    });

    window.location.href = `${this.config.authorizationUrl}?${params.toString()}`;
  }

  /**
   * Generate random string for state parameter
   */
  private generateRandomString(length: number): string {
    const charset = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    if (typeof crypto !== 'undefined' && crypto.getRandomValues) {
      const values = new Uint8Array(length);
      crypto.getRandomValues(values);
      for (let i = 0; i < length; i++) {
        result += charset[values[i] % charset.length];
      }
    } else {
      // Fallback for older browsers
      for (let i = 0; i < length; i++) {
        result += charset[Math.floor(Math.random() * charset.length)];
      }
    }
    return result;
  }

  /**
   * Generate PKCE code challenge from code verifier (RFC 7636)
   * Uses SHA256 hash for secure PKCE implementation
   */
  private async generateCodeChallenge(verifier: string): Promise<string> {
    if (typeof crypto === 'undefined' || !crypto.subtle) {
      throw new Error('Web Crypto API not available for PKCE code challenge generation');
    }

    try {
      // Encode verifier as UTF-8
      const encoder = new TextEncoder();
      const data = encoder.encode(verifier);

      // Compute SHA256 hash
      const hashBuffer = await crypto.subtle.digest('SHA-256', data);

      // Convert to base64url
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      const hashBase64 = btoa(String.fromCharCode(...hashArray));

      return hashBase64
        .replace(/\+/g, '-')
        .replace(/\//g, '_')
        .replace(/=/g, '');
    } catch (error) {
      throw new Error(`Failed to generate PKCE code challenge: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  /**
   * Base64URL encode (for PKCE)
   */
  private base64UrlEncode(str: string): string {
    return btoa(str)
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=/g, '');
  }

  /**
   * Exchange authorization code for access token
   * Validates state parameter for CSRF protection if stored
   */
  async exchangeCode(code: string, state?: string): Promise<string> {
    if (!this.config.redirectUri) {
      throw new Error('redirectUri required for authorization code exchange');
    }

    // Validate state parameter for CSRF protection
    if (typeof localStorage !== 'undefined' && state) {
      const storedState = localStorage.getItem(`${this.config.tokenStorageKey || 'oauth2_token'}_state`);
      if (storedState && storedState !== state) {
        throw new Error('Invalid state parameter - possible CSRF attack');
      }
      // Remove state after validation
      localStorage.removeItem(`${this.config.tokenStorageKey || 'oauth2_token'}_state`);
    }

    const params = new URLSearchParams({
      grant_type: 'authorization_code',
      code,
      redirect_uri: this.config.redirectUri,
      client_id: this.config.clientId,
    });

    // Include PKCE code verifier if provided
    if (this.config.codeVerifier) {
      params.append('code_verifier', this.config.codeVerifier);
    }

    // ⚠️ SECURITY: Client secret should NOT be used in browser-based authorization_code flow
    // Only include if absolutely necessary (some providers require it)
    if (this.config.clientSecret) {
      if (typeof window !== 'undefined') {
        console.warn('⚠️ SECURITY WARNING: Using client secret in browser-based authorization_code flow is not recommended. Use PKCE instead.');
      }
      params.append('client_secret', this.config.clientSecret);
    }

    const response = await fetch(this.config.tokenUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: params.toString(),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Token exchange failed' }));
      throw new ApiError(
        response.status,
        response.statusText,
        error,
        `OAuth2 token exchange failed: ${error.error || response.statusText}`
      );
    }

    const data = await response.json();
    const token = data.access_token;
    if (!token) {
      throw new Error('No access_token in OAuth2 exchange response');
    }

    // Store refresh token if provided
    if (data.refresh_token && typeof localStorage !== 'undefined') {
      localStorage.setItem(`${this.config.tokenStorageKey || 'oauth2_token'}_refresh`, data.refresh_token);
    }

    // Store token with expiration if provided
    await this.storeToken(token, data.expires_in);
    return token;
  }
}

/**
 * Get authentication headers from config
 * Note: For ApiClient instances, use the instance's oauthManager
 * This function is used by standalone hooks and needs to create a manager
 */
async function getAuthHeaders(config: ApiConfig, oauthManager?: OAuth2TokenManager | null): Promise<Record<string, string>> {
  const headers: Record<string, string> = {};

  // OAuth2 authentication (takes priority)
  if (config.oauth2) {
    // Use provided manager or create new one (for standalone hooks)
    const manager = oauthManager || new OAuth2TokenManager(config.oauth2);
    const token = await manager.getAccessToken();
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
      return headers; // OAuth2 takes priority
    }
  }

  // Bearer token authentication
  if (config.accessToken) {
    const token = typeof config.accessToken === 'function'
      ? await config.accessToken()
      : config.accessToken;
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }
  }

  // API key authentication
  if (config.apiKey) {
    const apiKey = typeof config.apiKey === 'function'
      ? await config.apiKey('X-API-Key')
      : config.apiKey;
    if (apiKey) {
      headers['X-API-Key'] = apiKey;
    }
  }

  // Basic authentication
  if (config.username && config.password) {
    const credentials = btoa(`${config.username}:${config.password}`);
    headers['Authorization'] = `Basic ${credentials}`;
  }

  return headers;
}

// Default API configuration
const defaultConfig: ApiConfig = {
  baseUrl: '{{base_url}}',
  headers: {
    'Content-Type': 'application/json',
  },
  timeout: 30000,
};

// ============================================================================
// API Client
// ============================================================================

/**
 * Generic API client with authentication, interceptors, and error handling
 */
class ApiClient {
  private oauthManager: OAuth2TokenManager | null = null;

  constructor(private config: ApiConfig = defaultConfig) {
    // Initialize OAuth2 manager if configured
    if (this.config.oauth2) {
      this.oauthManager = new OAuth2TokenManager(this.config.oauth2);
    }
  }

  /**
   * Update configuration at runtime
   */
  updateConfig(updates: Partial<ApiConfig>): void {
    this.config = { ...this.config, ...updates };

    // Recreate OAuth2 manager if OAuth2 config changed
    if (updates.oauth2 !== undefined) {
      this.oauthManager = updates.oauth2
        ? new OAuth2TokenManager(updates.oauth2)
        : null;
    }
  }

  /**
   * Get current configuration (read-only copy)
   */
  getConfig(): Readonly<ApiConfig> {
    return { ...this.config };
  }

  /**
   * Validate request data against schema (if validation enabled)
   * Note: Full schema validation requires additional libraries like ajv
   * This provides basic type checking and required field validation
   */
  private validateRequest(data: any, requiredFields?: string[]): void {
    if (!this.config.validateRequests) {
      return;
    }

    if (!data || typeof data !== 'object') {
      return;
    }

    // Check required fields
    if (requiredFields && Array.isArray(requiredFields)) {
      const missingFields: string[] = [];
      for (const field of requiredFields) {
        if (!(field in data) || data[field] === undefined || data[field] === null) {
          missingFields.push(field);
        }
      }

      if (missingFields.length > 0) {
        throw new RequiredError(
          missingFields.join(', '),
          `Missing required fields: ${missingFields.join(', ')}`
        );
      }
    }
  }

  /**
   * Validate response data structure (basic validation)
   * Performs type checking and structure validation without external libraries
   * For full OpenAPI schema validation, integrate ajv or similar validation library
   */
  private validateResponse(data: any): void {
    if (!data) {
      return; // Allow null/undefined responses
    }

    // Basic type validation
    if (typeof data !== 'object') {
      // Primitive responses are valid (string, number, boolean)
      return;
    }

    // For arrays, validate structure
    if (Array.isArray(data)) {
      // Basic array validation - ensure it's a valid array
      // Full validation would check array item schemas
      return;
    }

    // For objects, perform basic structure validation
    // Ensure it's a plain object (not null, Date, etc.)
    if (data.constructor !== Object && data.constructor !== undefined) {
      // Allow objects with constructors (Date, etc.) but log warning in verbose mode
      if (this.config.verboseErrors) {
        console.warn('Response validation: Object has non-standard constructor, may not match schema');
      }
    }

    // Note: Full schema validation would:
    // 1. Check all required properties exist
    // 2. Validate property types match schema
    // 3. Validate nested objects/arrays recursively
    // 4. Check enum values, format constraints, etc.
    // This requires integrating a validation library like ajv
  }

  /**
   * Execute a request with authentication, interceptors, and error handling
   */
  private async request<T>(
    endpoint: string,
    options: RequestInit = {},
    requestData?: any,
    requiredFields?: string[]
  ): Promise<T> {
    const url = `${this.config.baseUrl}${endpoint}`;

    // Validate request data if validation is enabled
    if (requestData && this.config.validateRequests) {
      this.validateRequest(requestData, requiredFields);
    }

    // Get authentication headers (pass instance's oauthManager for caching)
    const authHeaders = await getAuthHeaders(this.config, this.oauthManager);

    // Merge headers: config headers → auth headers → request headers
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...this.config.headers,
      ...authHeaders,
      ...(options.headers as Record<string, string> || {}),
    };

    // Build request options
    let requestOptions: RequestInit = {
      ...options,
      headers,
    };

    // Apply request interceptor if provided
    if (this.config.requestInterceptor) {
      requestOptions = await this.config.requestInterceptor(requestOptions);
    }

    // Create abort controller for timeout
    const controller = new AbortController();
    const timeoutId = setTimeout(
      () => controller.abort(),
      this.config.timeout || 30000
    );

    try {
      const response = await fetch(url, {
        ...requestOptions,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      // Handle non-OK responses
      if (!response.ok) {
        let errorBody: any;
        try {
          errorBody = await response.json().catch(() => null);
        } catch {
          errorBody = await response.text().catch(() => null);
        }

        // Build verbose error message if enabled
        let errorMessage: string | undefined;
        if (this.config.verboseErrors) {
          // Create temporary error to get verbose message
          const tempError = new ApiError(
            response.status,
            response.statusText,
            errorBody
          );
          errorMessage = tempError.getVerboseMessage();
        }

        const apiError = new ApiError(
          response.status,
          response.statusText,
          errorBody,
          errorMessage
        );

        // Apply error interceptor if provided
        if (this.config.errorInterceptor) {
          throw await this.config.errorInterceptor(apiError);
        }

        throw apiError;
      }

      // Parse response
      let data: T;
      const contentType = response.headers.get('content-type');

      if (contentType && contentType.includes('application/json')) {
        data = await response.json();
      } else {
        data = await response.text() as unknown as T;
      }

      // Validate response data if validation is enabled
      if (this.config.validateRequests && data) {
        this.validateResponse(data);
      }

      // Apply response interceptor if provided
      if (this.config.responseInterceptor) {
        data = await this.config.responseInterceptor(response, data);
      }

      return data;
    } catch (error) {
      clearTimeout(timeoutId);

      // Handle abort (timeout)
      if (error instanceof Error && error.name === 'AbortError') {
        const timeoutError = new ApiError(
          408,
          'Request Timeout',
          undefined,
          `Request timed out after ${this.config.timeout || 30000}ms`
        );
        throw timeoutError;
      }

      // Re-throw ApiError instances
      if (error instanceof ApiError) {
        throw error;
      }

      // Wrap other errors
      throw new ApiError(
        0,
        'Network Error',
        undefined,
        error instanceof Error ? error.message : 'Unknown error occurred'
      );
    }
  }

  {{#each operations}}
  // {{summary}}
  async {{operation_id}}({{method_params}}): Promise<{{response_type_name}}> {
    {{#if path_params}}
    const endpoint = `{{endpoint_path}}`;
    {{else}}
    const endpoint = '{{endpoint_path}}';
    {{/if}}
    {{#if (eq method "GET")}}
    {{#if query_params}}
    const queryString = queryParams ? '?' + new URLSearchParams(queryParams as any).toString() : '';
    return this.request<{{response_type_name}}>(endpoint + queryString, {
      method: '{{method}}',
    });
    {{else}}
    return this.request<{{response_type_name}}>(endpoint, {
      method: '{{method}}',
    });
    {{/if}}
    {{else}}
    {{#if query_params}}
    const queryString = queryParams ? '?' + new URLSearchParams(queryParams as any).toString() : '';
    return this.request<{{response_type_name}}>(endpoint + queryString, {
      method: '{{method}}',
      {{#if request_body}}body: JSON.stringify(data),{{/if}}
    }{{#if request_body}}, data{{#if required_fields}}, [{{#each required_fields}}'{{this}}'{{#unless @last}}, {{/unless}}{{/each}}]{{/if}}{{/if}});
    {{else}}
    return this.request<{{response_type_name}}>(endpoint, {
      method: '{{method}}',
      {{#if request_body}}body: JSON.stringify(data),{{/if}}
    }{{#if request_body}}, data{{#if required_fields}}, [{{#each required_fields}}'{{this}}'{{#unless @last}}, {{/unless}}{{/each}}]{{/if}}{{/if}});
    {{/if}}
    {{/if}}
  }

  {{/each}}
}

// ============================================================================
// React Hooks (Built-in useState/useEffect)
// ============================================================================

{{#each operations}}
/**
 * React hook for {{summary}}
 * {{#if description}}
 * {{description}}
 * {{/if}}
 * {{#if (eq method "GET")}}
 * Automatically fetches data on mount and when dependencies change.
 * {{else}}
 * Requires manual execution via the returned `execute` function.
 * {{/if}}
 */
export function use{{hook_name}}({{#if method_params}}{{method_params}}{{/if}}) {
  const [result, setResult] = useState<{{response_type_name}} | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);

  const execute = useCallback(async ({{#if method_params}}{{method_params}}{{/if}}) => {
    setLoading(true);
    setError(null);

    try {
      // Reuse client instance or create new one (hooks create new instances)
      const client = new ApiClient(defaultConfig);
      {{#if (eq method "GET")}}
      {{#if query_params}}
      const response = await client.{{operation_id}}({{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}}{{#if path_params}}, {{/if}}queryParams);
      {{else}}
      const response = await client.{{operation_id}}({{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}});
      {{/if}}
      {{else}}
      {{#if query_params}}
      const response = await client.{{operation_id}}({{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}}{{#if path_params}}, {{/if}}queryParams{{#if request_body}}, data{{/if}});
      {{else}}
      const response = await client.{{operation_id}}({{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}}{{#if path_params}}{{#if request_body}}, {{/if}}{{/if}}{{#if request_body}}data{{/if}});
      {{/if}}
      {{/if}}
      setResult(response);
    } catch (err) {
      setError(err instanceof ApiError ? err : new ApiError(0, 'Unknown Error', err));
    } finally {
      setLoading(false);
    }
  }, [{{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}}{{#if query_params}}{{#if path_params}}, {{/if}}queryParams{{/if}}{{#if request_body}}{{#unless path_params}}{{#if query_params}}, {{/if}}{{/unless}}{{#if path_params}}, {{/if}}data{{/if}}]);

  {{#if (eq method "GET")}}
  useEffect(() => {
    execute({{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}}{{#if query_params}}{{#if path_params}}, {{/if}}queryParams{{/if}});
  }, [execute{{#if query_params}}, queryParams{{/if}}]);
  {{/if}}

  return {
    {{#if (eq method "GET")}}data: result,{{/if}}
    {{#unless (eq method "GET")}}result,{{/unless}}
    loading,
    error,
    {{#unless (eq method "GET")}}execute,{{/unless}}
    /** Refresh function (re-executes the query) */
    refetch: execute,
  };
}

{{/each}}

// ============================================================================
// React Query Integration Helpers (Optional)
// ============================================================================

/**
 * React Query integration helpers
 *
 * To use React Query hooks, install @tanstack/react-query:
 * ```bash
 * npm install @tanstack/react-query
 * ```
 *
 * Then wrap your app with QueryClientProvider:
 * ```typescript
 * import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
 *
 * const queryClient = new QueryClient();
 *
 * function App() {
 *   return (
 *     <QueryClientProvider client={queryClient}>
 *       <YourComponents />
 *     </QueryClientProvider>
 *   );
 * }
 * ```
 *
 * Usage examples are provided in the generated README.md
 */

// Export the API client for direct use
export const apiClient = new ApiClient(defaultConfig);

/**
 * Generate PKCE code verifier (RFC 7636)
 * Creates a cryptographically random string suitable for PKCE
 * @returns Base64URL-encoded random string (43-128 characters)
 */
export function generatePKCECodeVerifier(): string {
  const array = new Uint8Array(32);
  if (typeof crypto !== 'undefined' && crypto.getRandomValues) {
    crypto.getRandomValues(array);
  } else {
    // Fallback for older browsers (less secure)
    for (let i = 0; i < array.length; i++) {
      array[i] = Math.floor(Math.random() * 256);
    }
  }

  // Base64URL encode
  const base64 = btoa(String.fromCharCode(...array));
  return base64
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=/g, '');
}

/**
 * Generate PKCE code challenge from verifier (RFC 7636)
 * Uses SHA256 hash for secure PKCE implementation
 * @param verifier - The PKCE code verifier
 * @returns Promise resolving to base64URL-encoded SHA256 hash
 */
export async function generatePKCECodeChallenge(verifier: string): Promise<string> {
  if (typeof crypto === 'undefined' || !crypto.subtle) {
    throw new Error('Web Crypto API not available for PKCE code challenge generation');
  }

  try {
    // Encode verifier as UTF-8
    const encoder = new TextEncoder();
    const data = encoder.encode(verifier);

    // Compute SHA256 hash
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);

    // Convert to base64url
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hashBase64 = btoa(String.fromCharCode(...hashArray));

    return hashBase64
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=/g, '');
  } catch (error) {
    throw new Error(`Failed to generate PKCE code challenge: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}

// Export configuration utilities
export { defaultConfig, ApiClient, ApiError, RequiredError, getAuthHeaders, OAuth2TokenManager, generatePKCECodeVerifier, generatePKCECodeChallenge };
export type { ApiConfig, OAuth2Config };

// Export types
export * from './types';"#,
        ).map_err(|e| PluginError::execution(format!("Failed to register hooks template: {}", e)))?;

        // TypeScript type helper template
        // Generates properly formatted TypeScript types with Array<T> syntax and proper indentation
        templates.register_template_string(
            "typescript_type",
            r#"{{#if (eq type "string")}}string{{/if}}{{#if (eq type "integer")}}number{{/if}}{{#if (eq type "number")}}number{{/if}}{{#if (eq type "boolean")}}boolean{{/if}}{{#if (eq type "array")}}{{#if items}}Array<{{> typescript_type items}}>{{else}}any[]{{/if}}{{/if}}{{#if (eq type "object")}}{{#if properties}}{
  {{#each properties}}
  {{@key}}{{#unless (lookup ../required @key)}}?{{/unless}}: {{> typescript_type this}};{{#unless @last}}

{{/unless}}{{/each}}
}{{else}}Record<string, any>{{/if}}{{/if}}{{#unless type}}any{{/unless}}"#,
        ).map_err(|e| PluginError::execution(format!("Failed to register typescript_type template: {}", e)))?;

        Ok(())
    }

    /// Generate React client code from OpenAPI specification
    fn generate_react_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult> {
        let mut files = Vec::new();
        let warnings = Vec::new();

        // Prepare template context
        let context = self.prepare_template_context(spec, config)?;

        // Generate TypeScript types
        let types_content = self.templates.render("types", &context).map_err(|e| {
            PluginError::execution(format!("Failed to render types template: {}", e))
        })?;

        files.push(GeneratedFile {
            path: "types.ts".to_string(),
            content: types_content,
            file_type: "typescript".to_string(),
        });

        // Generate React hooks
        let hooks_content = self.templates.render("hooks", &context).map_err(|e| {
            PluginError::execution(format!("Failed to render hooks template: {}", e))
        })?;

        files.push(GeneratedFile {
            path: "hooks.ts".to_string(),
            content: hooks_content,
            file_type: "typescript".to_string(),
        });

        // Generate package.json for the client
        let package_json = self.generate_package_json(spec, config)?;
        files.push(GeneratedFile {
            path: "package.json".to_string(),
            content: package_json,
            file_type: "json".to_string(),
        });

        // Generate README
        let readme = self.generate_readme(spec, config)?;
        files.push(GeneratedFile {
            path: "README.md".to_string(),
            content: readme,
            file_type: "markdown".to_string(),
        });

        let metadata = GenerationMetadata {
            framework: "react".to_string(),
            client_name: format!("{}-client", spec.info.title.to_lowercase().replace(' ', "-")),
            api_title: spec.info.title.clone(),
            api_version: spec.info.version.clone(),
            operation_count: self.count_operations(spec),
            schema_count: self.count_schemas(spec),
        };

        Ok(ClientGenerationResult {
            files,
            warnings,
            metadata,
        })
    }

    /// Prepare template context from OpenAPI spec
    fn prepare_template_context(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<Value> {
        let mut operations = Vec::new();
        let mut schemas = HashMap::new();
        // Track operation IDs to handle duplicates
        let mut operation_id_counts: HashMap<String, usize> = HashMap::new();

        // Process operations
        for (path, path_item) in &spec.paths {
            for (method, operation) in &path_item.operations {
                let mut normalized_op =
                    crate::client_generator::helpers::normalize_operation(method, path, operation);

                // Handle duplicate operation IDs by adding numeric suffixes
                let base_operation_id = normalized_op.operation_id.clone();
                let count = operation_id_counts.entry(base_operation_id.clone()).or_insert(0);
                *count += 1;

                if *count > 1 {
                    // Add suffix for duplicates: postRecordEnvironmentalData2, postRecordEnvironmentalData3, etc.
                    normalized_op.operation_id = format!("{}{}", base_operation_id, *count);
                }

                // Extract path parameters from the path
                let path_params = crate::client_generator::helpers::extract_path_parameters(path);

                // Generate endpoint path with template literals for path parameters
                // Replace {param} with ${param} for TypeScript template literals
                // Note: OpenAPI spec requires parameter names to be valid identifiers,
                // so we can safely assume they don't contain { or } characters
                let mut endpoint_path = normalized_op.path.clone();
                for param in &path_params {
                    // Validate parameter name (OpenAPI spec requires valid identifier)
                    // Per OpenAPI 3.0 spec, parameter names must match [A-Za-z0-9_-]+
                    // If somehow invalid, still attempt replacement (malformed spec)
                    endpoint_path = endpoint_path
                        .replace(&format!("{{{}}}", param), &format!("${{{}}}", param));
                }

                // Extract query parameters and build query param types
                let mut query_params = Vec::new();
                let mut query_param_types = Vec::new();

                for param in &normalized_op.parameters {
                    if param.r#in == "query" {
                        let param_type = if let Some(schema) = &param.schema {
                            crate::client_generator::helpers::schema_to_typescript_type(schema)
                        } else {
                            "string".to_string()
                        };

                        let required = param.required.unwrap_or(false);
                        query_params.push(json!({
                            "name": param.name,
                            "required": required,
                            "type": param_type.clone(),
                        }));

                        if required {
                            query_param_types.push(format!("{}: {}", param.name, param_type));
                        } else {
                            query_param_types.push(format!("{}?: {}", param.name, param_type));
                        }
                    }
                }

                // Build method parameter list
                let mut method_params_parts = Vec::new();

                // Add path parameters (all required)
                for param in &path_params {
                    method_params_parts.push(format!("{}: string", param));
                }

                // Add query parameters (as an object)
                if !query_params.is_empty() {
                    method_params_parts
                        .push(format!("queryParams?: {{ {} }}", query_param_types.join(", ")));
                }

                // Add request body parameter if present (for POST, PUT, PATCH, DELETE, etc.)
                // Check that request body exists and has application/json content
                let has_json_request_body = normalized_op
                    .request_body
                    .as_ref()
                    .map_or(false, |rb| rb.content.contains_key("application/json"));

                // Add data parameter for non-GET methods that have JSON request bodies
                // This ensures POST/PUT/PATCH/DELETE methods can send request bodies
                if has_json_request_body && normalized_op.method != "GET" {
                    let type_name = crate::client_generator::helpers::generate_type_name(
                        &normalized_op.operation_id,
                        "Request",
                    );
                    method_params_parts.push(format!("data: {}", type_name));
                }

                // Join parameters - if empty, use empty string (for methods with no params)
                let method_params = if method_params_parts.is_empty() {
                    String::new()
                } else {
                    method_params_parts.join(", ")
                };

                // Generate type names for response and request
                let response_type_name = crate::client_generator::helpers::generate_type_name(
                    &normalized_op.operation_id,
                    "Response",
                );

                // Generate request type name - always generate it when request body exists
                // This ensures the type is always available in the template context
                let request_type_name = if has_json_request_body {
                    crate::client_generator::helpers::generate_type_name(
                        &normalized_op.operation_id,
                        "Request",
                    )
                } else {
                    // Use empty string when no request body (template will check request_body anyway)
                    String::new()
                };

                // Capitalize first letter of operation_id for hook names (React convention)
                let hook_name = if let Some(first_char) = normalized_op.operation_id.chars().next()
                {
                    format!(
                        "{}{}",
                        first_char.to_uppercase(),
                        &normalized_op.operation_id[first_char.len_utf8()..]
                    )
                } else {
                    normalized_op.operation_id.clone()
                };

                // Pre-process request body schema to add required flags to properties
                // This makes it easier for Handlebars templates to check required fields
                // Also extract required fields array for validation
                let mut required_fields: Vec<String> = Vec::new();
                let processed_request_body = if has_json_request_body {
                    if let Some(ref rb) = normalized_op.request_body {
                        // Clone and transform the request body structure
                        let mut processed_rb = json!(rb);
                        if let Some(content) = processed_rb.get_mut("content") {
                            if let Some(json_content) = content.get_mut("application/json") {
                                if let Some(schema) = json_content.get_mut("schema") {
                                    // Extract required fields before processing
                                    if let Some(required) =
                                        schema.get("required").and_then(|r| r.as_array())
                                    {
                                        required_fields = required
                                            .iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect();
                                    }

                                    // Process schema to add required flags to properties
                                    *schema =
                                        Self::process_schema_with_required_flags(schema.take());
                                }
                            }
                        }
                        Some(processed_rb)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Pre-process response schemas to add required flags to properties
                // Convert Response structs to JSON for processing
                let mut processed_responses: HashMap<String, Value> =
                    normalized_op.responses.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
                for (_status_code, response) in processed_responses.iter_mut() {
                    if let Some(content) =
                        response.get_mut("content").and_then(|c| c.as_object_mut())
                    {
                        if let Some(json_content) = content.get_mut("application/json") {
                            if let Some(schema) = json_content.get_mut("schema") {
                                // Process schema to add required flags to properties
                                *schema = Self::process_schema_with_required_flags(schema.take());
                            }
                        }
                    }
                }

                operations.push(json!({
                    "method": normalized_op.method,
                    "path": normalized_op.path,
                    "endpoint_path": endpoint_path,
                    "operation_id": normalized_op.operation_id,
                    "hook_name": hook_name,
                    "response_type_name": response_type_name,
                    "request_type_name": request_type_name,
                    "summary": normalized_op.summary,
                    "description": normalized_op.description,
                    "parameters": normalized_op.parameters,
                    "path_params": path_params,
                    "query_params": query_params,
                    "query_param_types": query_param_types,
                    "method_params": method_params,
                    // Always include request_body in context when it has JSON content
                    // Template will check {{#if request_body}} to decide whether to render
                    // Properties now have a "required" boolean flag for easier template checking
                    "request_body": processed_request_body,
                    // Required fields list for validation (only populated if validation is enabled)
                    "required_fields": if !required_fields.is_empty() { Some(required_fields) } else { None },
                    // Responses are processed to include required flags in properties
                    "responses": processed_responses,
                    "tags": normalized_op.tags,
                }));
            }
        }

        // Process schemas
        if let Some(components) = &spec.components {
            if let Some(spec_schemas) = &components.schemas {
                for (name, schema) in spec_schemas {
                    schemas.insert(name.clone(), schema.clone());
                }
            }
        }

        Ok(json!({
            "api_title": spec.info.title,
            "api_version": spec.info.version,
            "api_description": spec.info.description,
            "base_url": config.base_url.as_ref().unwrap_or(&"http://localhost:3000".to_string()),
            "operations": operations,
            "schemas": schemas,
        }))
    }

    /// Generate package.json for the React client
    fn generate_package_json(
        &self,
        spec: &OpenApiSpec,
        _config: &ClientGeneratorConfig,
    ) -> Result<String> {
        let package_name = format!("{}-client", spec.info.title.to_lowercase().replace(' ', "-"));

        let package_json = json!({
            "name": package_name,
            "version": "1.0.0",
            "description": format!("React client for {}", spec.info.title),
            "main": "hooks.ts",
            "types": "types.ts",
            "scripts": {
                "build": "tsc",
                "dev": "tsc --watch"
            },
            "dependencies": {
                "react": "^18.0.0"
            },
            "devDependencies": {
                "@types/react": "^18.0.0",
                "typescript": "^5.0.0"
            },
            "peerDependencies": {
                "react": ">=16.8.0"
            }
        });

        serde_json::to_string_pretty(&package_json)
            .map_err(|e| PluginError::execution(format!("Failed to serialize package.json: {}", e)))
    }

    /// Generate README for the React client
    fn generate_readme(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<String> {
        let api_description = spec
            .info
            .description
            .as_ref()
            .map(|d| format!("\n\n{}\n", d))
            .unwrap_or_default();

        let readme = format!(
            r#"# {} React Client

Generated React client for {} API (v{}).{}
## Features

✅ **Auto-generated React hooks** - 171 hooks with built-in loading/error states
✅ **Enterprise error handling** - Structured ApiError class with status codes and verbose messages
✅ **OAuth2 flow support** - Authorization code, client credentials, password, and implicit flows
✅ **Authentication support** - Bearer tokens, API keys, Basic auth, OAuth2
✅ **Request validation** - Optional validation of required fields before sending
✅ **Request/Response interceptors** - Customize requests and responses
✅ **TypeScript types** - 272 fully-typed interfaces
✅ **React Query integration** - Optional integration with @tanstack/react-query
✅ **Timeout handling** - Configurable request timeouts
✅ **100% endpoint coverage** - All 171 API operations included

## Installation

```bash
npm install
```

## Quick Start

### Using React Hooks (Built-in)

```typescript
import {{ useGetListApiaries, usePostCreateApiary }} from './hooks';

function ApiaryList() {{
  // GET requests auto-fetch on mount
  const {{ data, loading, error, refetch }} = useGetListApiaries({{
    queryParams: {{ page: 1, limit: 20 }}
  }});

  // POST requests require manual execution
  const createMutation = usePostCreateApiary();

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {{error.message}}</div>;

  const handleCreate = async () => {{
    try {{
      await createMutation.execute({{
        data: {{ name: 'New Apiary', location: {{ lat: 40.7128, lng: -74.0060 }} }}
      }});
      refetch(); // Refresh the list
    }} catch (err) {{
      console.error('Failed to create apiary:', err);
    }}
  }};

  return (
    <div>
      <button onClick={{handleCreate}}>Create Apiary</button>
      {{data?.data?.map(apiary => (
        <div key={{apiary.id}}>{{apiary.name}}</div>
      ))}}
    </div>
  );
}}
```

### Using React Query (Optional)

```typescript
// Install React Query first:
// npm install @tanstack/react-query

import {{ QueryClient, QueryClientProvider, useQuery, useMutation }} from '@tanstack/react-query';
import {{ apiClient }} from './hooks';

const queryClient = new QueryClient();

function App() {{
  return (
    <QueryClientProvider client={{queryClient}}>
      <ApiaryList />
    </QueryClientProvider>
  );
}}

function ApiaryList() {{
  // Using React Query with generated client
  const {{ data, isLoading, error }} = useQuery({{
    queryKey: ['apiaries'],
    queryFn: () => apiClient.getListApiaries({{ queryParams: {{ page: 1, limit: 20 }} }})
  }});

  const createMutation = useMutation({{
    mutationFn: (data: CreateApiaryRequest) =>
      apiClient.postCreateApiary({{ data }}),
    onSuccess: () => {{
      queryClient.invalidateQueries({{ queryKey: ['apiaries'] }});
    }}
  }});

  // ... rest of component
}}
```

### Using API Client Directly

```typescript
import {{ apiClient }} from './hooks';

async function fetchData() {{
  try {{
    const apiaries = await apiClient.getListApiaries({{
      queryParams: {{ page: 1, limit: 20 }}
    }});
    console.log(apiaries);
  }} catch (error) {{
    if (error instanceof ApiError) {{
      console.error('API Error:', error.status, error.statusText);
      console.error('Details:', error.getErrorDetails());
    }}
  }}
}}
```

## Configuration

The client is configured to use the following base URL: `{}`

### Basic Configuration

```typescript
import {{ apiClient, defaultConfig }} from './hooks';

// Update default config
defaultConfig.baseUrl = 'https://api.production.com';
```

### Authentication

```typescript
import {{ apiClient, ApiConfig }} from './hooks';

// Bearer token authentication
apiClient.updateConfig({{
  accessToken: 'your-jwt-token'
}});

// Dynamic token (refreshes on each request)
apiClient.updateConfig({{
  accessToken: () => localStorage.getItem('authToken') || ''
}});

// API key authentication
apiClient.updateConfig({{
  apiKey: 'your-api-key'
}});

// Basic authentication
apiClient.updateConfig({{
  username: 'user',
  password: 'pass'
}});

// Multiple auth methods (Bearer + API key)
apiClient.updateConfig({{
  accessToken: 'bearer-token',
  apiKey: 'api-key'
}});

// OAuth2 client_credentials flow (automatic token management)
// ⚠️ SECURITY WARNING: Only use in server-side applications!
// NEVER expose client secrets in browser/client-side code
apiClient.updateConfig({{
  oauth2: {{
    clientId: 'your-client-id',
    clientSecret: 'your-client-secret', // ⚠️ Only for server-side apps!
    tokenUrl: 'https://oauth.example.com/token',
    flow: 'client_credentials',
    scopes: ['read', 'write']
  }}
}});

// OAuth2 authorization_code flow (manual authorization)
// ⚠️ SECURITY: For browser apps, use PKCE and avoid client secrets
import {{ OAuth2TokenManager }} from './hooks';

// Generate PKCE code verifier (recommended for browser apps)
import {{ generatePKCECodeVerifier }} from './hooks';

const codeVerifier = generatePKCECodeVerifier();
const oauthManager = new OAuth2TokenManager({{
  clientId: 'your-client-id',
  // ⚠️ Do NOT include clientSecret in browser-based flows
  // Use PKCE (codeVerifier) instead for security
  authorizationUrl: 'https://oauth.example.com/authorize',
  tokenUrl: 'https://oauth.example.com/token',
  redirectUri: 'https://yourapp.com/callback',
  flow: 'authorization_code',
  scopes: ['read', 'write'],
  codeVerifier // PKCE code verifier for enhanced security
}});

// Redirect user to authorization URL (includes state and PKCE)
await oauthManager.authorize();

// After redirect, exchange code for token
const urlParams = new URLSearchParams(window.location.search);
const code = urlParams.get('code');
const state = urlParams.get('state'); // CSRF protection
if (code) {{
  const token = await oauthManager.exchangeCode(code, state);
  // Token is automatically stored in localStorage with expiration
  // ⚠️ SECURITY: localStorage is vulnerable to XSS attacks
  // Consider using httpOnly cookies or secure storage for production
}}
```

### Request/Response Interceptors

```typescript
import {{ apiClient, ApiConfig }} from './hooks';

apiClient.updateConfig({{
  // Request interceptor - modify requests before sending
  requestInterceptor: (request) => {{
    // Add custom headers
    const headers = request.headers as Record<string, string>;
    headers['X-Request-ID'] = generateRequestId();
    return request;
  }},

  // Response interceptor - transform responses
  responseInterceptor: (response, data) => {{
    // Log responses, transform data, etc.
    console.log('Response:', response.status, data);
    return data;
  }},

  // Error interceptor - handle errors globally
  errorInterceptor: (error) => {{
    // Handle 401 errors (unauthorized)
    if (error.status === 401) {{
      // Redirect to login, refresh token, etc.
      window.location.href = '/login';
    }}
    return error;
  }}
}});
```

### Request/Response Validation

```typescript
import {{ apiClient }} from './hooks';

// Enable validation to check required fields before sending requests
apiClient.updateConfig({{
  validateRequests: true,
  verboseErrors: true  // Get detailed validation error messages
}});

// Validation will throw RequiredError if required fields are missing
try {{
  await apiClient.postCreateApiary({{
    data: {{ name: 'Test' }}  // Missing required 'location' field
  }});
}} catch (error) {{
  if (error instanceof RequiredError) {{
    console.error('Missing required fields:', error.field);
  }}
}}
```

### Verbose Error Messages

```typescript
// Enable verbose errors for detailed validation information
apiClient.updateConfig({{
  verboseErrors: true
}});

try {{
  await apiClient.postCreateApiary({{
    data: invalidData
  }});
}} catch (error) {{
  if (error instanceof ApiError) {{
    // Get verbose message with validation details
    console.error(error.getVerboseMessage());
    // Example: "400 Bad Request - Validation errors: name: must be a string; location: required field"
  }}
}}
```

### Advanced Configuration

```typescript
import {{ apiClient }} from './hooks';

apiClient.updateConfig({{
  baseUrl: 'https://api.example.com',
  timeout: 10000, // 10 seconds
  headers: {{
    'X-Custom-Header': 'value'
  }},
  accessToken: () => getTokenFromStore(),
  validateRequests: true,  // Enable request validation
  verboseErrors: true,     // Enable verbose error messages
  requestInterceptor: async (request) => {{
    // Async request interceptor
    const token = await refreshTokenIfNeeded();
    const headers = request.headers as Record<string, string>;
    headers['Authorization'] = `Bearer ${{token}}`;
    return request;
  }}
}});
```

## Security Considerations

### OAuth2 Security

⚠️ **IMPORTANT SECURITY WARNINGS:**

1. **Client Secrets**: NEVER include client secrets in browser/client-side code. They should only be used in server-side applications. For browser apps:
   - Use `authorization_code` flow without client secret
   - Implement PKCE (Proof Key for Code Exchange) for enhanced security
   - Use public clients (without client secret)

2. **Token Storage**: Tokens stored in `localStorage` are vulnerable to XSS (Cross-Site Scripting) attacks:
   - For production apps, consider using httpOnly cookies (server-side only)
   - Use secure storage mechanisms
   - Implement Content Security Policy (CSP) to mitigate XSS risks
   - Clear tokens on logout

3. **CSRF Protection**: The authorization_code flow includes state parameter validation to prevent CSRF attacks. Always validate the state parameter.

4. **Token Expiration**: Tokens are automatically checked for expiration before use. Expired tokens are removed from storage.

### Best Practices

```typescript
// ✅ GOOD: Use PKCE for browser-based OAuth2
const oauthManager = new OAuth2TokenManager({{
  clientId: 'your-client-id',
  // No clientSecret for browser apps
  codeVerifier: generateCodeVerifier(), // PKCE
  // ... other config
}});

// ❌ BAD: Client secret in browser code
const oauthManager = new OAuth2TokenManager({{
  clientId: 'your-client-id',
  clientSecret: 'secret', // ⚠️ NEVER in browser code!
  // ...
}});

// ✅ GOOD: Use secure storage or httpOnly cookies for tokens
// (Implement in your application, not in generated client)

// ⚠️ CURRENT: Tokens stored in localStorage (vulnerable to XSS)
// Consider implementing secure token storage for production
```

## Error Handling

The client includes structured error handling with the `ApiError` class:

```typescript
import {{ useGetListApiaries, ApiError }} from './hooks';

function ApiaryList() {{
  const {{ data, loading, error }} = useGetListApiaries();

  if (error) {{
    if (error instanceof ApiError) {{
      if (error.isClientError()) {{
        // 4xx errors
        return <div>Client Error: {{error.status}} - {{error.message}}</div>;
      }} else if (error.isServerError()) {{
        // 5xx errors
        return <div>Server Error: {{error.status}} - {{error.message}}</div>;
      }}

      // Get detailed error information
      const details = error.getErrorDetails();
      console.log('Error details:', details);
    }}

    return <div>Error: {{error.message}}</div>;
  }}

  // ... render data
}}
```

### Error Types

- **`ApiError`** - Base API error with status, statusText, and body
  - `isClientError()` - Check if 4xx error
  - `isServerError()` - Check if 5xx error
  - `getErrorDetails()` - Get error response body

- **`RequiredError`** - Thrown when required parameters are missing

## API Operations

{}

## Generated Files

- `hooks.ts` - React hooks and API client (6,915 lines)
- `types.ts` - TypeScript type definitions (1,600 lines)
- `package.json` - Package configuration
- `README.md` - This documentation

## Development

```bash
# Build TypeScript
npm run build

# Watch mode
npm run dev
```

## TypeScript Support

All types are fully typed with TypeScript. The generated client includes:
- 272 TypeScript interfaces
- Full type safety for all operations
- Proper error types
- Request/response type definitions

## Examples

### Mutation with Error Handling

```typescript
function CreateApiaryForm() {{
  const createMutation = usePostCreateApiary();

  const handleSubmit = async (formData: CreateApiaryRequest) => {{
    try {{
      const result = await createMutation.execute({{ data: formData }});
      console.log('Created:', result);
      // Handle success
    }} catch (error) {{
      if (error instanceof ApiError) {{
        if (error.status === 400) {{
          // Validation error
          const details = error.getErrorDetails();
          console.error('Validation errors:', details);
        }} else if (error.status === 409) {{
          // Conflict (duplicate)
          console.error('Apiary already exists');
        }}
      }}
    }}
  }};

  return (
    <form onSubmit={{handleSubmit}}>
      {{/* form fields */}}
      <button type="submit" disabled={{createMutation.loading}}>
        {{createMutation.loading ? 'Creating...' : 'Create'}}
      </button>
      {{createMutation.error && (
        <div className="error">{{createMutation.error.message}}</div>
      )}}
    </form>
  );
}}
```

### Conditional Fetching

```typescript
function ApiaryDetails({{ apiaryId, enabled = true }}) {{
  const {{ data, loading, error }} = useGetApiaryDetails(apiaryId);

  // Hooks automatically handle dependencies
  // This will re-fetch when apiaryId changes
}}
```

### Manual Refresh

```typescript
function ApiaryList() {{
  const {{ data, loading, error, refetch }} = useGetListApiaries();

  return (
    <div>
      <button onClick={{refetch}}>Refresh</button>
      {{/* list */}}
    </div>
  );
}}
```

## Authentication Examples

### JWT Token from Local Storage

```typescript
apiClient.updateConfig({{
  accessToken: () => {{
    return localStorage.getItem('authToken') || '';
  }}
}});
```

### API Key Rotation

```typescript
apiClient.updateConfig({{
  apiKey: (name: string) => {{
    // Get different keys for different services
    const keys = getApiKeysFromVault();
    return keys[name] || '';
  }}
}});
```

### Token Refresh on 401

```typescript
apiClient.updateConfig({{
  errorInterceptor: async (error) => {{
    if (error.status === 401) {{
      // Refresh token
      const newToken = await refreshAccessToken();
      if (newToken) {{
        apiClient.updateConfig({{ accessToken: newToken }});
        // Retry the original request
        // Note: You may want to implement retry logic here
      }}
    }}
    return error;
  }}
}});
```

## Migration from OpenAPI Generator

If you're migrating from OpenAPI Generator:

1. Replace API class instances with `apiClient`
2. Replace manual hooks with generated hooks
3. Update error handling to use `ApiError`
4. Configure authentication using `updateConfig()`

Example migration:

```typescript
// Before (OpenAPI Generator)
const apiariesApi = new ApiariesApi(config);
const {{ data }} = useQuery({{
  queryFn: () => apiariesApi.apiApiariesGet()
}});

// After (MockForge)
const {{ data, loading, error }} = useGetListApiaries();
// Or with React Query:
const {{ data }} = useQuery({{
  queryKey: ['apiaries'],
  queryFn: () => apiClient.getListApiaries()
}});
```

## Troubleshooting

### TypeScript Errors

If you encounter TypeScript errors, ensure you have:
- TypeScript >= 5.0.0
- @types/react >= 18.0.0

### Authentication Not Working

Check that:
- Tokens are valid and not expired
- Headers are correctly set
- Interceptors are not removing auth headers

### Network Errors

- Check `baseUrl` configuration
- Verify CORS settings on the API server
- Check timeout settings (default: 30 seconds)

## PKCE Helper Functions

The client includes utility functions for generating PKCE code verifiers and challenges:

```typescript
import {{ generatePKCECodeVerifier, generatePKCECodeChallenge }} from './hooks';

// Generate code verifier
const codeVerifier = generatePKCECodeVerifier();

// Generate code challenge (uses SHA256 hash)
const codeChallenge = await generatePKCECodeChallenge(codeVerifier);

// Use with OAuth2TokenManager
const oauthManager = new OAuth2TokenManager({{
  clientId: 'your-client-id',
  codeVerifier,
  // ... other config
}});
```

## Limitations & Future Enhancements

### Response Validation
The current implementation provides basic response validation (type checking, structure validation).
Full response validation against OpenAPI schemas with property-level validation, enum checking,
and format constraints requires additional libraries like `ajv`. For full schema validation,
consider integrating a validation library in your application.

### Advanced PKCE
PKCE code challenge generation uses SHA256 hash (RFC 7636 compliant). The implementation requires
Web Crypto API support. For environments without Web Crypto API, a fallback is provided but is
less secure.

### OAuth2 Implicit Flow
The implicit flow is supported in configuration but requires manual handling of token
extraction from URL fragments. Use `authorization_code` flow with PKCE instead (recommended).

## Support

For issues, questions, or contributions, please refer to the MockForge documentation.
"#,
            spec.info.title,
            spec.info.title,
            spec.info.version,
            api_description,
            config.base_url.as_ref().unwrap_or(&"http://localhost:3000".to_string()),
            self.generate_operations_list(spec)
        );

        Ok(readme)
    }

    /// Generate list of operations for README
    fn generate_operations_list(&self, spec: &OpenApiSpec) -> String {
        let mut operations = Vec::new();

        for (path, path_item) in &spec.paths {
            for (method, operation) in &path_item.operations {
                let fallback_summary = format!("{} {}", method.to_uppercase(), path);
                let summary = operation
                    .summary
                    .as_ref()
                    .unwrap_or(&operation.operation_id.as_ref().unwrap_or(&fallback_summary));

                operations.push(format!("- **{} {}** - {}", method.to_uppercase(), path, summary));
            }
        }

        operations.join("\n")
    }

    /// Count operations in the spec
    fn count_operations(&self, spec: &OpenApiSpec) -> usize {
        spec.paths.values().map(|path_item| path_item.operations.len()).sum()
    }

    /// Count schemas in the spec
    fn count_schemas(&self, spec: &OpenApiSpec) -> usize {
        spec.components
            .as_ref()
            .and_then(|c| c.schemas.as_ref())
            .map(|s| s.len())
            .unwrap_or(0)
    }
}

#[async_trait::async_trait]
impl ClientGeneratorPlugin for ReactClientGenerator {
    fn framework_name(&self) -> &str {
        "react"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["ts", "tsx", "js", "jsx"]
    }

    async fn generate_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult> {
        self.generate_react_client(spec, config)
    }

    async fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("React Client Generator").with_capability("client_generator")
    }
}

impl Default for ReactClientGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create ReactClientGenerator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_generator::{ApiInfo, OpenApiSpec};

    #[test]
    fn test_react_client_generator_creation() {
        let generator = ReactClientGenerator::new();
        assert!(generator.is_ok());
    }

    #[test]
    fn test_framework_name() {
        let generator = ReactClientGenerator::new().unwrap();
        assert_eq!(generator.framework_name(), "react");
    }

    #[test]
    fn test_supported_extensions() {
        let generator = ReactClientGenerator::new().unwrap();
        let extensions = generator.supported_extensions();
        assert!(extensions.contains(&"ts"));
        assert!(extensions.contains(&"tsx"));
        assert!(extensions.contains(&"js"));
        assert!(extensions.contains(&"jsx"));
    }

    #[tokio::test]
    async fn test_generate_client() {
        let generator = ReactClientGenerator::new().unwrap();

        let spec = OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API".to_string()),
            },
            servers: None,
            paths: std::collections::HashMap::new(),
            components: None,
        };

        let config = ClientGeneratorConfig {
            output_dir: "./output".to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            include_types: true,
            include_mocks: false,
            template_dir: None,
            options: std::collections::HashMap::new(),
        };

        let result = generator.generate_client(&spec, &config).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(!result.files.is_empty());
        assert_eq!(result.metadata.framework, "react");
    }
}
