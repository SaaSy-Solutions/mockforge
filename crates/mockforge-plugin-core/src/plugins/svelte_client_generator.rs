//! Svelte Client Generator Plugin
//!
//! Generates Svelte stores and TypeScript types from OpenAPI specifications
//! for easy integration with Svelte applications.

use crate::client_generator::{
    ClientGenerationResult, ClientGeneratorConfig, ClientGeneratorPlugin, GeneratedFile,
    GenerationMetadata, OpenApiSpec,
};
use crate::types::{PluginError, PluginMetadata, Result};
use handlebars::Handlebars;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Svelte client generator plugin
pub struct SvelteClientGenerator {
    /// Template registry for code generation
    templates: Handlebars<'static>,
}

impl SvelteClientGenerator {
    /// Create a new Svelte client generator
    pub fn new() -> Result<Self> {
        let mut templates = Handlebars::new();

        // Register templates for Svelte code generation
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
                        Value::Bool(required_fields.contains(prop_name)),
                    );
                }
            }
        }
        schema
    }

    /// Register Handlebars templates for Svelte code generation
    fn register_templates(templates: &mut Handlebars<'static>) -> Result<()> {
        // Register JSON helper for serializing schemas
        templates.register_helper(
            "json",
            Box::new(
                |h: &handlebars::Helper,
                 _: &Handlebars,
                 _: &handlebars::Context,
                 _: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let value = h.param(0).ok_or_else(|| {
                        handlebars::RenderError::new("json helper requires a parameter")
                    })?;
                    let json_str = serde_json::to_string(&value.value()).map_err(|e| {
                        handlebars::RenderError::new(format!("Failed to serialize to JSON: {}", e))
                    })?;
                    out.write(&json_str)?;
                    Ok(())
                },
            ),
        );

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
export interface {{operation_id}}Response {
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
{{> typescript_type this.schema}}
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
export interface {{operation_id}}Request {
{{#each request_body.content}}
{{#if (eq @key "application/json")}}
{{#if this.schema}}
{{#if this.schema.properties}}
{{#each this.schema.properties}}
  {{@key}}{{#unless this.required}}?{{/unless}}: {{> typescript_type this}};
{{/each}}
{{else}}
{{> typescript_type this.schema}}
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

        // Svelte stores template - Enhanced with all production features
        // This template includes all infrastructure from React/Vue (ApiError, TokenStorage, OAuth2, etc.)
        // but uses Svelte's reactive stores (writable, derived) instead of Vue/React patterns
        templates.register_template_string(
            "stores",
            r#"// Generated Svelte stores for {{api_title}}
// API Version: {{api_version}}

import { writable, derived, type Writable } from 'svelte/store';

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

/**
 * Contract validation error with schema path and contract diff reference
 *
 * This error type provides detailed information about validation failures
 * and can link back to contract diff entries for tracking breaking changes.
 */
export class ContractValidationError extends ApiError {
  constructor(
    status: number,
    statusText: string,
    public schemaPath: string,
    public expectedType: string,
    public actualValue?: any,
    public contractDiffId?: string,
    public isBreakingChange: boolean = false,
    body?: any,
    message?: string
  ) {
    super(
      status,
      statusText,
      body,
      message || `Contract validation failed at '${schemaPath}': expected ${expectedType}${actualValue !== undefined ? `, got ${JSON.stringify(actualValue)}` : ''}`
    );
    this.name = 'ContractValidationError';
    Object.setPrototypeOf(this, ContractValidationError.prototype);
  }

  /**
   * Get a detailed error message with contract diff information
   */
  getDetailedMessage(): string {
    let msg = `Validation failed at '${this.schemaPath}': expected ${this.expectedType}`;
    if (this.actualValue !== undefined) {
      msg += `, got ${typeof this.actualValue === 'object' ? JSON.stringify(this.actualValue) : String(this.actualValue)}`;
    }
    if (this.contractDiffId) {
      msg += ` (Contract Diff ID: ${this.contractDiffId})`;
    }
    if (this.isBreakingChange) {
      msg += ' [BREAKING CHANGE]';
    }
    return msg;
  }
}

// ============================================================================
// Token Storage Interface
// ============================================================================

/**
 * Token storage interface for secure token management
 * Allows different storage backends (localStorage, httpOnly cookies, secure storage)
 */
export interface TokenStorage {
  /** Get access token from storage */
  getAccessToken(): string | null | Promise<string | null>;
  /** Store access token with optional expiration (in seconds) */
  setAccessToken(token: string, expiresIn?: number): void | Promise<void>;
  /** Get refresh token from storage */
  getRefreshToken(): string | null | Promise<string | null>;
  /** Store refresh token */
  setRefreshToken(token: string): void | Promise<void>;
  /** Clear all tokens from storage */
  clearTokens(): void | Promise<void>;
}

/**
 * LocalStorage-based token storage implementation
 * ⚠️ SECURITY: localStorage is vulnerable to XSS attacks
 * Consider using httpOnly cookies or secure storage for production apps
 */
export class LocalStorageTokenStorage implements TokenStorage {
  private accessTokenKey: string;
  private refreshTokenKey: string;

  constructor(
    accessTokenKey: string = 'access_token',
    refreshTokenKey: string = 'refresh_token'
  ) {
    this.accessTokenKey = accessTokenKey;
    this.refreshTokenKey = refreshTokenKey;
  }

  getAccessToken(): string | null {
    if (typeof localStorage === 'undefined') {
      return null;
    }

    const stored = localStorage.getItem(this.accessTokenKey);
    if (!stored) return null;

    try {
      // Try to parse as JSON (with expiration) or use as plain string
      const parsed = JSON.parse(stored);
      if (parsed.token && parsed.expiresAt) {
        // Check if token is expired
        if (Date.now() >= parsed.expiresAt * 1000) {
          localStorage.removeItem(this.accessTokenKey);
          return null;
        }
        return parsed.token;
      }
      // Legacy format (plain string) - return as token
      return typeof parsed === 'string' ? parsed : parsed.token || parsed;
    } catch {
      // Plain string format
      return stored;
    }
  }

  setAccessToken(token: string, expiresIn?: number): void {
    if (typeof localStorage === 'undefined') {
      return;
    }

    // Store token with expiration if provided (expiresIn is in seconds)
    const tokenData = expiresIn
      ? JSON.stringify({
          token,
          expiresAt: Math.floor(Date.now() / 1000) + expiresIn,
        })
      : token;
    localStorage.setItem(this.accessTokenKey, tokenData);
  }

  getRefreshToken(): string | null {
    if (typeof localStorage === 'undefined') {
      return null;
    }
    return localStorage.getItem(this.refreshTokenKey);
  }

  setRefreshToken(token: string): void {
    if (typeof localStorage === 'undefined') {
      return;
    }
    localStorage.setItem(this.refreshTokenKey, token);
  }

  clearTokens(): void {
    if (typeof localStorage === 'undefined') {
      return;
    }
    localStorage.removeItem(this.accessTokenKey);
    localStorage.removeItem(this.refreshTokenKey);
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
 * JWT Token Configuration
 * Handles JWT token refresh on 401 errors
 */
export interface JwtConfig {
  /** Refresh endpoint URL (default: '/api/v1/auth/refresh') */
  refreshEndpoint?: string;
  /** Refresh token (static or dynamic function) */
  refreshToken?: string | (() => string | Promise<string>);
  /** Callback invoked when token is refreshed */
  onTokenRefresh?: (token: string) => void | Promise<void>;
  /** Callback invoked when authentication fails (refresh token invalid/expired) */
  onAuthError?: () => void | Promise<void>;
  /** Refresh token if it expires within this many seconds (default: 300) */
  refreshThreshold?: number;
  /** Check token expiration before making requests (default: true) */
  checkExpirationBeforeRequest?: boolean;
}

/**
 * Retry Configuration
 * Configures automatic retry behavior for failed requests
 */
export interface RetryConfig {
  /** Maximum number of retry attempts (default: 3) */
  maxRetries?: number;
  /** Base delay in milliseconds for exponential backoff (default: 1000) */
  baseDelay?: number;
  /** Maximum delay in milliseconds (default: 10000) */
  maxDelay?: number;
  /** HTTP status codes that should be retried (default: [408, 429, 500, 502, 503, 504]) */
  retryableStatusCodes?: number[];
  /** Whether to retry on network errors (default: true) */
  retryOnNetworkError?: boolean;
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
  /** JWT token configuration for automatic refresh on 401 */
  jwt?: JwtConfig;
  /** Retry configuration for automatic retry on failures */
  retry?: RetryConfig;
  /** Token storage implementation (default: LocalStorageTokenStorage) */
  tokenStorage?: TokenStorage;
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
  /** Enable response validation (default: false) */
  validateResponses?: boolean;
  /** Enable verbose error messages (default: false) */
  verboseErrors?: boolean;
  /** Automatically unwrap ApiResponse<T> format to return data directly (default: false) */
  unwrapResponse?: boolean;
  /** Schema registry for runtime validation (schema_id -> JSON Schema) */
  schemas?: Record<string, any>;
  /** Whether to include contract diff references in validation errors */
  includeContractDiffs?: boolean;
}

/**
 * OAuth2 Token Manager
 * Handles OAuth2 flows and token refresh
 */
class OAuth2TokenManager {
  private tokenStorage: TokenStorage;

  constructor(
    private config: OAuth2Config,
    tokenStorage?: TokenStorage
  ) {
    // Use provided token storage or create one with OAuth2-specific keys
    if (tokenStorage) {
      this.tokenStorage = tokenStorage;
    } else {
      const storageKey = this.config.tokenStorageKey || 'oauth2_token';
      this.tokenStorage = new LocalStorageTokenStorage(
        storageKey,
        `${storageKey}_refresh`
      );
    }
  }

  /**
   * Get stored access token with expiration check
   * ⚠️ SECURITY: Tokens in localStorage are vulnerable to XSS attacks
   */
  private getStoredToken(): { token: string; expiresAt?: number } | null {
    const token = this.tokenStorage.getAccessToken();
    if (!token) return null;
    return { token };
  }

  /**
   * Store access token with optional expiration
   * ⚠️ SECURITY: Tokens stored in localStorage are vulnerable to XSS attacks
   * Consider using httpOnly cookies or secure storage for production apps
   */
  private async storeToken(token: string, expiresIn?: number): Promise<void> {
    await this.tokenStorage.setAccessToken(token, expiresIn);
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
      // Store state for CSRF validation (using localStorage directly for state, not tokens)
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
    if (data.refresh_token) {
      await this.tokenStorage.setRefreshToken(data.refresh_token);
    }

    // Store token with expiration if provided
    await this.storeToken(token, data.expires_in);
    return token;
  }
}

/**
 * Get authentication headers from config
 * Note: For ApiClient instances, use the instance's oauthManager
 * This function is used by standalone stores and needs to create a manager
 */
async function getAuthHeaders(config: ApiConfig, oauthManager?: OAuth2TokenManager | null): Promise<Record<string, string>> {
  const headers: Record<string, string> = {};

  // OAuth2 authentication (takes priority)
  if (config.oauth2) {
    // Use provided manager or create new one (for standalone stores)
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

// ============================================================================
// Base URL Resolver (Frictionless Drop-In Mode)
// ============================================================================

/**
 * MockForge mode for endpoint switching
 */
export type MockForgeMode = 'mock' | 'real' | 'hybrid';

/**
 * Base URL resolver that supports environment-based switching between mock and real endpoints
 *
 * Environment variables:
 * - MOCKFORGE_MODE: 'mock' | 'real' | 'hybrid' (default: uses explicit config)
 * - MOCKFORGE_BASE_URL: Override base URL (default: uses explicit config)
 * - MOCKFORGE_REALITY_LEVEL: 0.0-1.0 for hybrid mode (0.0 = 100% mock, 1.0 = 100% real)
 */
export class BaseUrlResolver {
  static resolve(
    mockBaseUrl: string,
    realBaseUrl: string,
    explicitBaseUrl?: string
  ): string {
    const envBaseUrl = this.getEnvVar('MOCKFORGE_BASE_URL');
    if (envBaseUrl) return envBaseUrl;
    if (explicitBaseUrl) return explicitBaseUrl;
    const mode = this.getMode();
    switch (mode) {
      case 'mock': return mockBaseUrl;
      case 'real': return realBaseUrl;
      case 'hybrid': return mockBaseUrl;
      default: return mockBaseUrl;
    }
  }
  static getMode(): MockForgeMode | null {
    const mode = this.getEnvVar('MOCKFORGE_MODE');
    if (!mode) return null;
    const normalized = mode.toLowerCase().trim();
    if (normalized === 'mock' || normalized === 'real' || normalized === 'hybrid') {
      return normalized as MockForgeMode;
    }
    return null;
  }
  static getRealityLevel(): number | null {
    const level = this.getEnvVar('MOCKFORGE_REALITY_LEVEL');
    if (!level) return null;
    const parsed = parseFloat(level);
    if (isNaN(parsed) || parsed < 0 || parsed > 1) return null;
    return parsed;
  }
  private static getEnvVar(name: string): string | null {
    if (typeof process !== 'undefined' && process.env) {
      if (process.env[name]) return process.env[name];
      const prefixes = ['VITE_', 'REACT_APP_', 'NEXT_PUBLIC_', 'NUXT_PUBLIC_'];
      for (const prefix of prefixes) {
        const prefixedName = prefix + name;
        if (process.env[prefixedName]) return process.env[prefixedName];
      }
    }
    if (typeof window !== 'undefined') {
      // @ts-ignore
      if (window.__ENV__ && window.__ENV__[name]) return window.__ENV__[name];
      const metaTag = document.querySelector(`meta[name="${name}"]`);
      if (metaTag) {
        const content = metaTag.getAttribute('content');
        if (content) return content;
      }
    }
    return null;
  }
}

// Bundled schemas for runtime validation
// These schemas are used when validateRequests or validateResponses is enabled
// Install ajv for full schema validation: npm install ajv
const bundledSchemas: Record<string, any> = {{#if bundled_schemas}}{
  {{#each bundled_schemas}}
  '{{@key}}': {{json this}},
  {{/each}}
}{{else}}{}{{/if}};

// Default API configuration
const defaultConfig: ApiConfig = {
  baseUrl: BaseUrlResolver.resolve(
    '{{base_url}}',
    '{{real_base_url}}',
    undefined
  ),
  headers: {
    'Content-Type': 'application/json',
  },
  timeout: 30000,
  schemas: bundledSchemas, // Include bundled schemas for runtime validation
  includeContractDiffs: true, // Enable contract diff references in errors
};

// ============================================================================
// API Client
// ============================================================================

/**
 * Generic API client with authentication, interceptors, and error handling
 */
class ApiClient {
  private oauthManager: OAuth2TokenManager | null = null;
  private tokenStorage: TokenStorage;
  private refreshPromise: Promise<string> | null = null;
  private pendingRequests: Array<{
    resolve: (token: string) => void;
    reject: (error: Error) => void;
  }> = [];

  constructor(private config: ApiConfig = defaultConfig) {
    // Initialize token storage (default to LocalStorageTokenStorage)
    this.tokenStorage = this.config.tokenStorage || new LocalStorageTokenStorage();

    // Initialize OAuth2 manager if configured (share token storage if available)
    if (this.config.oauth2) {
      this.oauthManager = new OAuth2TokenManager(this.config.oauth2, this.tokenStorage);
    }
  }

  /**
   * Update configuration at runtime
   */
  updateConfig(updates: Partial<ApiConfig>): void {
    this.config = { ...this.config, ...updates };

    // Update token storage if provided
    if (updates.tokenStorage !== undefined) {
      this.tokenStorage = updates.tokenStorage;
    }

    // Recreate OAuth2 manager if OAuth2 config changed (share token storage)
    if (updates.oauth2 !== undefined) {
      this.oauthManager = updates.oauth2
        ? new OAuth2TokenManager(updates.oauth2, this.tokenStorage)
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
   * Supports both basic validation (required fields) and full JSON Schema validation
   */
  private validateRequest(data: any, requiredFields?: string[], schemaId?: string): void {
    if (!this.config.validateRequests) {
      return;
    }

    if (!data || typeof data !== 'object') {
      return;
    }

    // Check required fields (basic validation)
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

    // Full JSON Schema validation (if schema provided and ajv available)
    if (schemaId && this.config.schemas && this.config.schemas[schemaId]) {
      this.validateAgainstSchema(data, this.config.schemas[schemaId], schemaId, 'request');
    }
  }

  /**
   * Validate data against JSON Schema using ajv (if available)
   * Falls back to basic validation if ajv is not available
   */
  private validateAgainstSchema(
    data: any,
    schema: any,
    schemaId: string,
    context: 'request' | 'response'
  ): void {
    // Try to use ajv if available (user must install: npm install ajv)
    if (typeof window !== 'undefined' && (window as any).ajv) {
      const Ajv = (window as any).ajv;
      const ajv = new Ajv({ allErrors: true, strict: false });
      const validate = ajv.compile(schema);
      const valid = validate(data);

      if (!valid && validate.errors) {
        const firstError = validate.errors[0];
        const schemaPath = firstError.instancePath || firstError.schemaPath || '';
        const expectedType = firstError.schema?.type || firstError.params?.type || 'unknown';
        const actualValue = firstError.data;

        // Try to get contract diff ID from schema metadata
        const contractDiffId = schema['x-contract-diff-id'] || schema.contractDiffId;
        const isBreakingChange = schema['x-breaking-change'] || schema.isBreakingChange || false;

        throw new ContractValidationError(
          400,
          'Validation Error',
          schemaPath || `${context}.${schemaId}`,
          expectedType,
          actualValue,
          contractDiffId,
          isBreakingChange,
          { errors: validate.errors },
          `Schema validation failed for ${context}`
        );
      }
    } else if (typeof require !== 'undefined') {
      // Node.js environment - try to require ajv
      try {
        const Ajv = require('ajv');
        const ajv = new Ajv({ allErrors: true, strict: false });
        const validate = ajv.compile(schema);
        const valid = validate(data);

        if (!valid && validate.errors) {
          const firstError = validate.errors[0];
          const schemaPath = firstError.instancePath || firstError.schemaPath || '';
          const expectedType = firstError.schema?.type || firstError.params?.type || 'unknown';
          const actualValue = firstError.data;
          const contractDiffId = schema['x-contract-diff-id'] || schema.contractDiffId;
          const isBreakingChange = schema['x-breaking-change'] || schema.isBreakingChange || false;

          throw new ContractValidationError(
            400,
            'Validation Error',
            schemaPath || `${context}.${schemaId}`,
            expectedType,
            actualValue,
            contractDiffId,
            isBreakingChange,
            { errors: validate.errors },
            `Schema validation failed for ${context}`
          );
        }
      } catch (e) {
        // ajv not available - fall back to basic validation
        console.warn('ajv not available, using basic validation only. Install ajv for full schema validation: npm install ajv');
      }
    }
  }

  /**
   * Validate response data against schema (if validation enabled)
   */
  private validateResponse(data: any, schemaId?: string): void {
    if (!this.config.validateResponses) {
      return;
    }

    if (!data) {
      return;
    }

    // Full JSON Schema validation (if schema provided)
    if (schemaId && this.config.schemas && this.config.schemas[schemaId]) {
      this.validateAgainstSchema(data, this.config.schemas[schemaId], schemaId, 'response');
    }
  }

  /**
   * Check if token is expired or will expire soon
   * Returns true if token should be refreshed
   */
  private async shouldRefreshToken(token: string | null): Promise<boolean> {
    if (!token || !this.config.jwt?.checkExpirationBeforeRequest) {
      return false;
    }

    try {
      // Try to decode JWT exp claim (basic base64 decode, no verification)
      const parts = token.split('.');
      if (parts.length !== 3) return false;

      const payload = JSON.parse(atob(parts[1]));
      if (payload.exp) {
        const expiresAt = payload.exp * 1000; // Convert to milliseconds
        const threshold = (this.config.jwt.refreshThreshold || 300) * 1000;
        return Date.now() + threshold >= expiresAt;
      }
    } catch {
      // If we can't decode, tokenStorage.getAccessToken() already handles expiration
      // Return false as we can't determine expiration from JWT payload
      return false;
    }

    return false;
  }

  /**
   * Refresh JWT token using refresh token
   * Implements promise deduplication to prevent concurrent refresh requests
   * Supports ApiResponse<T> wrapper format and both camelCase/snake_case token formats
   */
  private async refreshJwtToken(): Promise<string> {
    // If refresh is already in progress, return the existing promise
    if (this.refreshPromise) {
      return this.refreshPromise;
    }

    // Create new refresh promise
    this.refreshPromise = (async () => {
      try {
        const jwtConfig = this.config.jwt;
        if (!jwtConfig) {
          throw new Error('JWT configuration not found');
        }

        // Get refresh token
        const refreshTokenValue = typeof jwtConfig.refreshToken === 'function'
          ? await jwtConfig.refreshToken()
          : jwtConfig.refreshToken || await Promise.resolve(this.tokenStorage.getRefreshToken());

        if (!refreshTokenValue) {
          throw new Error('Refresh token not available');
        }

        // Get refresh endpoint
        const refreshEndpoint = jwtConfig.refreshEndpoint || '/api/v1/auth/refresh';
        const refreshUrl = `${this.config.baseUrl}${refreshEndpoint}`;

        // Make refresh request
        const response = await fetch(refreshUrl, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            ...this.config.headers,
          },
          body: JSON.stringify({ refreshToken: refreshTokenValue }),
        });

        if (!response.ok) {
          const errorBody = await response.json().catch(() => ({}));
          throw new ApiError(
            response.status,
            response.statusText,
            errorBody,
            `JWT refresh failed: ${response.status} ${response.statusText}`
          );
        }

        // Parse response (support ApiResponse wrapper and direct format)
        const responseData = await response.json();
        let tokenData: any;

        // Check if response is wrapped in ApiResponse format
        if (responseData.success === true && responseData.data) {
          tokenData = responseData.data;
        } else {
          tokenData = responseData;
        }

        // Extract tokens (support both camelCase and snake_case)
        const accessToken = tokenData.accessToken || tokenData.access_token;
        const refreshToken = tokenData.refreshToken || tokenData.refresh_token;
        const expiresIn = tokenData.expiresIn || tokenData.expires_in;

        if (!accessToken) {
          throw new Error('No access token in refresh response');
        }

        // Store tokens
        await this.tokenStorage.setAccessToken(accessToken, expiresIn);
        if (refreshToken) {
          await this.tokenStorage.setRefreshToken(refreshToken);
        }

        // Call onTokenRefresh callback if provided
        if (jwtConfig.onTokenRefresh) {
          await jwtConfig.onTokenRefresh(accessToken);
        }

        // Resolve all pending requests
        this.pendingRequests.forEach(({ resolve }) => resolve(accessToken));
        this.pendingRequests = [];

        return accessToken;
      } catch (error) {
        // Clear tokens on failure
        await this.tokenStorage.clearTokens();

        // Call onAuthError callback if provided
        if (this.config.jwt?.onAuthError) {
          await this.config.jwt.onAuthError();
        }

        // Reject all pending requests
        this.pendingRequests.forEach(({ reject }) => reject(error instanceof Error ? error : new Error(String(error))));
        this.pendingRequests = [];

        throw error;
      } finally {
        // Clear refresh promise
        this.refreshPromise = null;
      }
    })();

    return this.refreshPromise;
  }

  /**
   * Wait for token refresh to complete (for queued requests)
   */
  private async waitForTokenRefresh(): Promise<string> {
    if (this.refreshPromise) {
      return this.refreshPromise;
    }

    // If no refresh in progress, create a promise that will be resolved/rejected by refresh
    return new Promise<string>((resolve, reject) => {
      this.pendingRequests.push({ resolve, reject });
    });
  }

  /**
   * Unwrap ApiResponse<T> format if configured
   * Supports both wrapped and unwrapped responses for backward compatibility
   */
  private unwrapApiResponse<T>(data: any): T {
    if (!this.config.unwrapResponse) {
      return data;
    }

    // Check if response matches ApiResponse<T> format
    if (data && typeof data === 'object' && data.success === true && 'data' in data) {
      return data.data;
    }

    // Return as-is if not wrapped
    return data;
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
   * Calculate exponential backoff delay with jitter
   */
  private calculateBackoffDelay(retryCount: number): number {
    const retryConfig = this.config.retry || {};
    const baseDelay = retryConfig.baseDelay || 1000;
    const maxDelay = retryConfig.maxDelay || 10000;

    // Exponential backoff: baseDelay * 2^retryCount
    const exponentialDelay = Math.min(baseDelay * Math.pow(2, retryCount), maxDelay);

    // Add jitter: random(0, 0.3 * delay)
    const jitter = Math.random() * 0.3 * exponentialDelay;

    return Math.floor(exponentialDelay + jitter);
  }

  /**
   * Check if status code is retryable
   */
  private isRetryableStatusCode(status: number): boolean {
    const retryConfig = this.config.retry || {};
    const retryableStatusCodes = retryConfig.retryableStatusCodes || [408, 429, 500, 502, 503, 504];
    return retryableStatusCodes.includes(status);
  }

  /**
   * Check if error is a network error
   */
  private isNetworkError(error: any): boolean {
    if (!(error instanceof Error)) return false;
    return error instanceof TypeError && (
      error.message.includes('fetch') ||
      error.message.includes('network') ||
      error.message.includes('Failed to fetch')
    );
  }

  /**
   * Execute a request with authentication, interceptors, retry logic, and error handling
   */
  private async request<T>(
    endpoint: string,
    options: RequestInit = {},
    requestData?: any,
    requiredFields?: string[]
  ): Promise<T> {
    const url = `${this.config.baseUrl}${endpoint}`;
    const retryConfig = this.config.retry || {};
    const maxRetries = retryConfig.maxRetries ?? 3;
    let retryCount = 0;
    let lastError: Error | null = null;

    // Validate request data if validation is enabled
    if (requestData && this.config.validateRequests) {
      // Try to determine schema ID from endpoint context
      const schemaId = endpoint.split('/').pop()?.replace(/\{|\}/g, '') + 'Request';
      this.validateRequest(requestData, requiredFields, schemaId);
    }

    // Helper function to execute a single request attempt
    const executeRequest = async (): Promise<T> => {
      // Check token expiration before request (proactive refresh)
      if (this.config.jwt?.checkExpirationBeforeRequest) {
        const currentToken = await Promise.resolve(this.tokenStorage.getAccessToken());
        if (await this.shouldRefreshToken(currentToken)) {
          try {
            await this.refreshJwtToken();
          } catch (error) {
            // If proactive refresh fails, continue with request (will trigger 401 refresh)
            console.warn('Proactive token refresh failed, continuing with request:', error);
          }
        }
      }

      // Get authentication headers (pass instance's oauthManager for caching)
      const authHeaders = await getAuthHeaders(this.config, this.oauthManager);

      // If using JWT, get token from storage and add to headers
      if (this.config.jwt && !authHeaders['Authorization']) {
        const token = await Promise.resolve(this.tokenStorage.getAccessToken());
        if (token) {
          authHeaders['Authorization'] = `Bearer ${token}`;
        }
      }

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

          // Handle ApiErrorResponse format
          if (errorBody && typeof errorBody === 'object' && errorBody.success === false && errorBody.error) {
            errorBody = errorBody.error;
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

          // Handle 401 errors with JWT refresh
          if (response.status === 401 && this.config.jwt) {
            try {
              // Refresh token
              await this.refreshJwtToken();

              // Retry original request with new token (do not count as retry)
              return executeRequest();
            } catch (refreshError) {
              // Refresh failed, throw auth error
              if (this.config.jwt.onAuthError) {
                await this.config.jwt.onAuthError();
              }
              throw apiError;
            }
          }

          // Apply error interceptor if provided (before retry logic)
          if (this.config.errorInterceptor) {
            const interceptedError = await this.config.errorInterceptor(apiError);
            throw interceptedError;
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

        // Unwrap ApiResponse format if configured
        data = this.unwrapApiResponse(data);

        // Validate response data if validation is enabled
        if (this.config.validateResponses && data) {
          // Try to determine schema ID from endpoint/operation
          const schemaId = endpoint.split('/').pop()?.replace(/\{|\}/g, '') + 'Response';
          this.validateResponse(data, schemaId);
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

        // Store error for retry logic
        lastError = error instanceof Error ? error : new Error(String(error));

        // Re-throw ApiError instances (will be caught by retry logic if retryable)
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
    };

    // Retry loop
    while (retryCount <= maxRetries) {
      try {
        return await executeRequest();
      } catch (error) {
        // If this is the last retry attempt, throw the error
        if (retryCount >= maxRetries) {
          throw error;
        }

        // Check if error is retryable
        const isRetryable = error instanceof ApiError
          ? this.isRetryableStatusCode(error.status)
          : (retryConfig.retryOnNetworkError !== false && this.isNetworkError(error));

        // Don't retry non-retryable errors
        if (!isRetryable) {
          throw error;
        }

        // Don't retry 401 errors (handled by JWT refresh)
        if (error instanceof ApiError && error.status === 401) {
          throw error;
        }

        // Don't retry 403 errors (authorization failure)
        if (error instanceof ApiError && error.status === 403) {
          throw error;
        }

        // Calculate backoff delay
        const delay = this.calculateBackoffDelay(retryCount);

        // Wait before retrying
        await new Promise(resolve => setTimeout(resolve, delay));

        retryCount++;
      }
    }

    // Should never reach here, but TypeScript needs it
    throw lastError || new Error('Request failed after retries');
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
// Scenario-First SDKs
// ============================================================================

/**
 * Scenario execution result
 */
export interface ScenarioExecutionResult {
  scenarioId: string;
  success: boolean;
  stepResults: Array<{
    stepId: string;
    success: boolean;
    statusCode?: number;
    responseBody?: any;
    extractedVariables: Record<string, any>;
    error?: string;
    durationMs: number;
  }>;
  durationMs: number;
  error?: string;
  finalState: Record<string, any>;
}

/**
 * Scenario executor for high-level business workflows
 *
 * Enables executing scenarios like "CheckoutSuccess" that chain multiple
 * API calls together, instead of manually calling individual endpoints.
 */
export class ScenarioExecutor {
  constructor(private apiClient: ApiClient) {}

  /**
   * Execute a scenario by ID
   */
  async executeScenario(
    scenarioId: string,
    parameters?: Record<string, any>
  ): Promise<ScenarioExecutionResult> {
    throw new Error(
      `Scenario '${scenarioId}' not found. ` +
      `Scenarios must be registered via the scenario registry or defined in the SDK.`
    );
  }

  /**
   * Register a scenario definition
   */
  async registerScenario(scenario: {
    id: string;
    name: string;
    description?: string;
    steps: Array<{
      id: string;
      name: string;
      method: string;
      path: string;
      body?: any;
      extract?: Record<string, string>;
      expectedStatus?: number;
    }>;
    parameters?: Array<{
      name: string;
      type: string;
      required?: boolean;
    }>;
  }): Promise<void> {
    console.warn('Scenario registration not yet implemented. Scenarios should be defined at SDK generation time.');
  }
}

// ============================================================================
// Svelte Stores (Built-in writable/derived)
// ============================================================================

// Singleton API client instance (shared across all stores)
const apiClient = new ApiClient(defaultConfig);

// Export scenario executor
export const scenarioExecutor = new ScenarioExecutor(apiClient);

// Global configuration store (reactive)
export const apiConfig: Writable<ApiConfig> = writable(defaultConfig);

// Update API client when config changes
apiConfig.subscribe(config => {
  apiClient.updateConfig(config);
});

{{#each operations}}
/**
 * Svelte store for {{summary}}
 * {{#if description}}
 * {{description}}
 * {{/if}}
 * {{#if (eq method "GET")}}
 * Automatically fetches data on initialization.
 * {{else}}
 * Requires manual execution via the returned `execute` function.
 * {{/if}}
 */
export const {{operation_id}}Store = (() => {
  const data = writable<{{response_type_name}} | null>(null);
  const loading = writable<boolean>(false);
  const error = writable<ApiError | null>(null);

  const execute = async ({{#if method_params}}{{method_params}}{{/if}}) => {
    loading.set(true);
    error.set(null);

    try {
      {{#if (eq method "GET")}}
      {{#if query_params}}
      const response = await apiClient.{{operation_id}}({{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}}{{#if path_params}}, {{/if}}queryParams);
      {{else}}
      const response = await apiClient.{{operation_id}}({{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}});
      {{/if}}
      {{else}}
      {{#if query_params}}
      const response = await apiClient.{{operation_id}}({{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}}{{#if path_params}}, {{/if}}queryParams{{#if request_body}}, data{{/if}});
      {{else}}
      const response = await apiClient.{{operation_id}}({{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}}{{#if path_params}}{{#if request_body}}, {{/if}}{{/if}}{{#if request_body}}data{{/if}});
      {{/if}}
      {{/if}}
      data.set(response);
    } catch (err) {
      const apiError = err instanceof ApiError ? err : new ApiError(0, 'Unknown Error', err);
      error.set(apiError);
    } finally {
      loading.set(false);
    }
  };

  {{#if (eq method "GET")}}
  // Auto-execute for GET requests
  execute({{#if path_params}}{{#each path_params}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}{{/if}}{{#if query_params}}{{#if path_params}}, {{/if}}queryParams{{/if}});
  {{/if}}

  return {
    data: derived(data, $data => $data),
    loading: derived(loading, $loading => $loading),
    error: derived(error, $error => $error),
    {{#unless (eq method "GET")}}execute,{{/unless}}
    /** Refresh function (re-executes the query) */
    refresh: execute,
  };
})();

{{/each}}

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
export { defaultConfig, ApiClient, ApiError, RequiredError, ContractValidationError, getAuthHeaders, OAuth2TokenManager, generatePKCECodeVerifier, generatePKCECodeChallenge, TokenStorage, LocalStorageTokenStorage, apiClient, ScenarioExecutor, scenarioExecutor, BaseUrlResolver };
export type { ApiConfig, OAuth2Config, JwtConfig, RetryConfig, MockForgeMode, ScenarioExecutionResult };

// Export types
export * from './types';"#,
        ).map_err(|e| PluginError::execution(format!("Failed to register stores template: {}", e)))?;

        // Svelte component template
        templates
            .register_template_string(
                "component",
                r#"<!-- Generated Svelte component for {{api_title}} -->
<!-- API Version: {{api_version}} -->

<script lang="ts">
  import \{ onMount \} from 'svelte';
  import \{ {{{operation_id}}}Store, type ApiError \} from './stores';
  import type \{ {{{operation_id}}}Response \} from './types';

  // Reactive variables using store subscriptions
  let data: {{{operation_id}}}Response | null = null;
  let loading: boolean = false;
  let error: ApiError | null = null;

  // Subscribe to stores
  {{{operation_id}}}Store.data.subscribe(value => data = value);
  {{{operation_id}}}Store.loading.subscribe(value => loading = value);
  {{{operation_id}}}Store.error.subscribe(value => error = value);

  // Component logic
  function handleRefresh() \{
    {{{operation_id}}}Store.refresh();
  \}

  // Helper to get error message
  function getErrorMessage(err: ApiError | null): string \{
    if (!err) return '';
    return err.getVerboseMessage ? err.getVerboseMessage() : err.message;
  \}
</script>

<div class="api-component">
  <h2>{{api_title}} API</h2>

  <div class="controls">
    <button on:click=\{handleRefresh\} disabled=\{loading\}>
      \{loading ? 'Loading...' : 'Refresh'\}
    </button>
  </div>

  \{#if loading\}
    <div class="loading">Loading...</div>
  \{:else if error\}
    <div class="error">
      <strong>Error \{error.status || 'Unknown'\}:</strong> \{getErrorMessage(error)\}
      \{#if error.getErrorDetails && error.getErrorDetails()\}
        <details>
          <summary>Error Details</summary>
          <pre>\{JSON.stringify(error.getErrorDetails(), null, 2)\}</pre>
        </details>
      \{/if\}
    </div>
  \{:else if data\}
    <div class="data">
      <pre>\{JSON.stringify(data, null, 2)\}</pre>
    </div>
  \{:else\}
    <div class="no-data">No data available</div>
  \{/if\}
</div>

<style>
  .api-component \{
    padding: 1rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    margin: 1rem 0;
  \}

  .controls \{
    margin-bottom: 1rem;
  \}

  .loading \{
    color: #666;
    font-style: italic;
  \}

  .error \{
    color: #d32f2f;
    background-color: #ffebee;
    padding: 0.5rem;
    border-radius: 4px;
  \}

  .error details \{
    margin-top: 0.5rem;
  \}

  .error pre \{
    margin-top: 0.5rem;
    font-size: 0.875rem;
    background-color: rgba(0, 0, 0, 0.05);
    padding: 0.5rem;
    border-radius: 4px;
  \}

  .data \{
    background-color: #f5f5f5;
    padding: 1rem;
    border-radius: 4px;
    overflow-x: auto;
  \}

  .no-data \{
    color: #666;
    font-style: italic;
  \}

  button \{
    padding: 0.5rem 1rem;
    background-color: #1976d2;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  \}

  button:disabled \{
    background-color: #ccc;
    cursor: not-allowed;
  \}
</style>"#,
            )
            .map_err(|e| {
                PluginError::execution(format!("Failed to register component template: {}", e))
            })?;

        // TypeScript type helper template
        templates.register_template_string(
            "typescript_type",
            r#"{{#if (eq type "string")}}string{{/if}}{{#if (eq type "integer")}}number{{/if}}{{#if (eq type "number")}}number{{/if}}{{#if (eq type "boolean")}}boolean{{/if}}{{#if (eq type "array")}}{{#if items}}{{> typescript_type items}}[]{{else}}any[]{{/if}}{{/if}}{{#if (eq type "object")}}{{#if properties}}{ {{#each properties}}{{@key}}: {{> typescript_type this}}{{#unless @last}}, {{/unless}}{{/each}} }{{else}}Record<string, any>{{/if}}{{/if}}{{#unless type}}any{{/unless}}"#,
        ).map_err(|e| PluginError::execution(format!("Failed to register typescript_type template: {}", e)))?;

        Ok(())
    }

    /// Generate Svelte client code from OpenAPI specification
    fn generate_svelte_client(
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

        // Generate Svelte stores
        let stores_content = self.templates.render("stores", &context).map_err(|e| {
            PluginError::execution(format!("Failed to render stores template: {}", e))
        })?;

        files.push(GeneratedFile {
            path: "stores.ts".to_string(),
            content: stores_content,
            file_type: "typescript".to_string(),
        });

        // Generate example Svelte component
        let component_content = self.templates.render("component", &context).map_err(|e| {
            PluginError::execution(format!("Failed to render component template: {}", e))
        })?;

        files.push(GeneratedFile {
            path: "ApiComponent.svelte".to_string(),
            content: component_content,
            file_type: "svelte".to_string(),
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
            framework: "svelte".to_string(),
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
        let mut operation_id_counts = HashMap::new();

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
                let mut endpoint_path = normalized_op.path.clone();
                for param in &path_params {
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

                // Check if request body has JSON content
                let has_json_request_body = normalized_op
                    .request_body
                    .as_ref()
                    .is_some_and(|rb| rb.content.contains_key("application/json"));

                // Add request body parameter if present (for POST, PUT, PATCH, DELETE, etc.)
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
                let request_type_name = if has_json_request_body {
                    crate::client_generator::helpers::generate_type_name(
                        &normalized_op.operation_id,
                        "Request",
                    )
                } else {
                    String::new()
                };

                // Capitalize first letter of operation_id for store names (Svelte convention)
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
                // Also extract required fields array for validation
                let mut required_fields: Vec<String> = Vec::new();
                let processed_request_body = if has_json_request_body {
                    if let Some(ref rb) = normalized_op.request_body {
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
                let mut processed_responses: HashMap<String, Value> =
                    normalized_op.responses.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
                for (_status_code, response) in processed_responses.iter_mut() {
                    if let Some(content) =
                        response.get_mut("content").and_then(|c| c.as_object_mut())
                    {
                        if let Some(json_content) = content.get_mut("application/json") {
                            if let Some(schema) = json_content.get_mut("schema") {
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
                    "request_body": processed_request_body,
                    "required_fields": if !required_fields.is_empty() { Some(required_fields) } else { None },
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

        // Extract real base URL from OpenAPI spec servers or use a default
        let real_base_url = spec
            .servers
            .as_ref()
            .and_then(|servers| servers.first())
            .map(|server| server.url.clone())
            .unwrap_or_else(|| "https://api.production.com".to_string());

        // Prepare schemas for bundling (convert to JSON Schema format for runtime validation)
        let mut bundled_schemas = serde_json::Map::new();
        for (schema_name, schema) in &schemas {
            // Convert OpenAPI schema to JSON Schema format
            // Add schema metadata for contract diff tracking
            let mut schema_json = json!(schema);
            if let Some(schema_obj) = schema_json.as_object_mut() {
                // Add schema ID for lookup
                schema_obj.insert("$id".to_string(), json!(schema_name));
                // Add metadata fields for contract diff tracking (can be populated by drift detection)
                schema_obj.insert("x-schema-id".to_string(), json!(schema_name));
            }
            bundled_schemas.insert(schema_name.clone(), schema_json);
        }

        Ok(json!({
            "api_title": spec.info.title,
            "api_version": spec.info.version,
            "api_description": spec.info.description,
            "base_url": config.base_url.as_ref().unwrap_or(&"http://localhost:3000".to_string()),
            "real_base_url": real_base_url,
            "operations": operations,
            "schemas": schemas,
            "bundled_schemas": bundled_schemas,
        }))
    }

    /// Generate package.json for the Svelte client
    fn generate_package_json(
        &self,
        spec: &OpenApiSpec,
        _config: &ClientGeneratorConfig,
    ) -> Result<String> {
        let package_name = format!("{}-client", spec.info.title.to_lowercase().replace(' ', "-"));

        let package_json = json!({
            "name": package_name,
            "version": "1.0.0",
            "description": format!("Svelte client for {}", spec.info.title),
            "main": "stores.ts",
            "types": "types.ts",
            "scripts": {
                "build": "rollup -c",
                "dev": "rollup -c -w",
                "start": "sirv public --no-cors"
            },
            "dependencies": {
                "svelte": "^4.0.0"
            },
            "devDependencies": {
                "@rollup/plugin-commonjs": "^17.0.0",
                "@rollup/plugin-node-resolve": "^11.0.0",
                "rollup": "^2.0.0",
                "rollup-plugin-css-only": "^3.0.0",
                "rollup-plugin-livereload": "^2.0.0",
                "rollup-plugin-svelte": "^7.0.0",
                "sirv-cli": "^2.0.0",
                "typescript": "^5.0.0"
            },
            "peerDependencies": {
                "svelte": ">=3.0.0"
            }
        });

        serde_json::to_string_pretty(&package_json)
            .map_err(|e| PluginError::execution(format!("Failed to serialize package.json: {}", e)))
    }

    /// Generate README for the Svelte client
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

        let operation_count = self.count_operations(spec);
        let schema_count = self.count_schemas(spec);
        let default_url = "http://localhost:3000".to_string();
        let base_url = config.base_url.as_ref().unwrap_or(&default_url);
        let operations_list = self.generate_operations_list(spec);

        let readme = format!(
            r#"# {} Svelte Client

Generated Svelte client for {} API (v{}).{}
## Features

✅ **Auto-generated Svelte stores** - {} reactive stores with built-in loading/error states
✅ **Enterprise error handling** - Structured ApiError class with status codes and verbose messages
✅ **OAuth2 flow support** - Authorization code, client credentials, password, and implicit flows
✅ **Authentication support** - Bearer tokens, API keys, Basic auth, OAuth2
✅ **JWT token refresh** - Automatic refresh on 401 with promise deduplication
✅ **Retry logic** - Exponential backoff with jitter for resilient requests
✅ **Token storage interface** - Secure, extensible token management
✅ **Request validation** - Optional validation of required fields before sending
✅ **Request/Response interceptors** - Customize requests and responses
✅ **TypeScript types** - {} fully-typed interfaces
✅ **Timeout handling** - Configurable request timeouts
✅ **ApiResponse wrapper** - Support for unwrapping wrapped API responses
✅ **100% endpoint coverage** - All {} API operations included

## Installation

```bash
npm install
```

## Quick Start

### Using Svelte Stores

```svelte
<script lang="ts">
  import {{ getUsersStore, type ApiError }} from './stores';
  import type {{ GetUsersResponse }} from './types';

  // Subscribe to stores (reactive)
  $: data = $getUsersStore.data;
  $: loading = $getUsersStore.loading;
  $: error = $getUsersStore.error;

  // Execute operations
  function handleRefresh() {{
    getUsersStore.refresh();
  }}

  // Error handling
  function getErrorMessage(err: ApiError | null): string {{
    if (!err) return '';
    return err.getVerboseMessage ? err.getVerboseMessage() : err.message;
  }}
</script>

<div>
  {{#if $loading}}
    <div>Loading...</div>
  {{:else if $error}}
    <div class="error">
      <strong>Error {{$error.status || 'Unknown'}}:</strong> {{getErrorMessage($error)}}
    </div>
  {{:else if $data}}
    <div>
      {{#each $data as item (item.id)}}
        <div>{{item.name}}</div>
      {{/each}}
    </div>
  {{/if}}
  <button on:click={{handleRefresh}} disabled={{$loading}}>Refresh</button>
</div>
```

### Using API Client Directly

```typescript
import {{ apiClient, ApiError }} from './stores';

async function fetchData() {{
  try {{
    const users = await apiClient.getUsers({{
      queryParams: {{ page: 1, limit: 20 }}
    }});
    console.log(users);
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
import {{ apiConfig, apiClient }} from './stores';
import {{ writable }} from 'svelte/store';

// Update config via reactive store
apiConfig.update(config => ({{
  ...config,
  baseUrl: 'https://api.production.com'
}}));

// Or update client directly
apiClient.updateConfig({{
  baseUrl: 'https://api.production.com'
}});
```

### Authentication

```typescript
import {{ apiClient }} from './stores';

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
```

### JWT Token Refresh

```typescript
import {{ apiClient }} from './stores';

// Configure JWT token refresh
apiClient.updateConfig({{
  jwt: {{
    refreshEndpoint: '/api/v1/auth/refresh',
    refreshToken: () => localStorage.getItem('refreshToken') || '',
    onTokenRefresh: (token) => {{
      console.log('Token refreshed:', token);
    }},
    onAuthError: () => {{
      // Redirect to login on auth failure
      window.location.href = '/login';
    }},
    refreshThreshold: 300, // Refresh if expires within 5 minutes
    checkExpirationBeforeRequest: true // Proactive refresh
  }}
}});
```

### Retry Logic

```typescript
import {{ apiClient }} from './stores';

// Configure retry behavior
apiClient.updateConfig({{
  retry: {{
    maxRetries: 3,
    baseDelay: 1000, // 1 second
    maxDelay: 10000, // 10 seconds
    retryableStatusCodes: [408, 429, 500, 502, 503, 504],
    retryOnNetworkError: true
  }}
}});
```

### Request/Response Interceptors

```typescript
import {{ apiClient, ApiError }} from './stores';

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
  errorInterceptor: (error: ApiError) => {{
    // Handle 401 errors (unauthorized)
    if (error.status === 401) {{
      // Redirect to login
      window.location.href = '/login';
    }}
    return error;
  }}
}});
```

### ApiResponse Wrapper

```typescript
import {{ apiClient }} from './stores';

// Enable automatic unwrapping of ApiResponse<T> format
apiClient.updateConfig({{
  unwrapResponse: true // Automatically unwrap {{ success: true, data: T }} to return T
}});
```

## Security Considerations

⚠️ **IMPORTANT SECURITY WARNINGS:**

1. **Token Storage**: The default `LocalStorageTokenStorage` uses localStorage, which is vulnerable to XSS attacks. For production apps, consider:
   - Using httpOnly cookies (server-side)
   - Using secure storage mechanisms
   - Implementing token encryption

2. **OAuth2 Client Secrets**: NEVER include client secrets in browser/client-side code. Only use:
   - `authorization_code` flow with PKCE (recommended for browser apps)
   - `client_credentials` flow only in server-side applications

3. **PKCE for OAuth2**: Always use PKCE (Proof Key for Code Exchange) for browser-based authorization_code flows:

```typescript
import {{ generatePKCECodeVerifier, generatePKCECodeChallenge }} from './stores';

// Generate PKCE code verifier
const codeVerifier = generatePKCECodeVerifier();
const codeChallenge = await generatePKCECodeChallenge(codeVerifier);

// Use in OAuth2 config
apiClient.updateConfig({{
  oauth2: {{
    clientId: 'your-client-id',
    authorizationUrl: 'https://oauth.example.com/authorize',
    tokenUrl: 'https://oauth.example.com/token',
    redirectUri: 'https://yourapp.com/callback',
    flow: 'authorization_code',
    codeVerifier // PKCE code verifier
  }}
}});
```

## Error Handling

```typescript
import {{ ApiError }} from './stores';

try {{
  await apiClient.getUsers();
}} catch (error) {{
  if (error instanceof ApiError) {{
    // Check error type
    if (error.isClientError()) {{
      // 4xx errors
      console.error('Client error:', error.status);
    }} else if (error.isServerError()) {{
      // 5xx errors
      console.error('Server error:', error.status);
    }}

    // Get detailed error message
    console.error('Verbose message:', error.getVerboseMessage());

    // Get error details
    const details = error.getErrorDetails();
    console.error('Error details:', details);
  }}
}}
```

## Generated Files

- `types.ts` - TypeScript type definitions ({} schemas)
- `stores.ts` - Svelte stores and API client ({} operations)
- `component.svelte` - Example Svelte component
- `package.json` - Package configuration
- `README.md` - This documentation

## API Operations

{}

## Development

```bash
# Build TypeScript
npm run build

# Watch mode
npm run dev

# Start development server
npm run start
```
"#,
            spec.info.title,
            spec.info.title,
            spec.info.version,
            api_description,
            operation_count,
            schema_count,
            operation_count,
            base_url,
            schema_count,
            operation_count,
            operations_list
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
                    .unwrap_or(operation.operation_id.as_ref().unwrap_or(&fallback_summary));

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
impl ClientGeneratorPlugin for SvelteClientGenerator {
    fn framework_name(&self) -> &str {
        "svelte"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["ts", "js", "svelte"]
    }

    async fn generate_client(
        &self,
        spec: &OpenApiSpec,
        config: &ClientGeneratorConfig,
    ) -> Result<ClientGenerationResult> {
        self.generate_svelte_client(spec, config)
    }

    async fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Svelte Client Generator").with_capability("client_generator")
    }
}

impl Default for SvelteClientGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create SvelteClientGenerator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_generator::{ApiInfo, OpenApiSpec};

    #[test]
    fn test_svelte_client_generator_creation() {
        let generator = SvelteClientGenerator::new();
        assert!(generator.is_ok());
    }

    #[test]
    fn test_framework_name() {
        let generator = SvelteClientGenerator::new().unwrap();
        assert_eq!(generator.framework_name(), "svelte");
    }

    #[test]
    fn test_supported_extensions() {
        let generator = SvelteClientGenerator::new().unwrap();
        let extensions = generator.supported_extensions();
        assert!(extensions.contains(&"ts"));
        assert!(extensions.contains(&"js"));
        assert!(extensions.contains(&"svelte"));
    }

    #[tokio::test]
    async fn test_generate_client() {
        let generator = SvelteClientGenerator::new().unwrap();

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
        assert_eq!(result.metadata.framework, "svelte");
    }
}
