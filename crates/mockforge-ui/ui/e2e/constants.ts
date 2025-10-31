/**
 * Constants for E2E tests
 * 
 * Centralized constants to reduce magic numbers and improve maintainability
 */

/**
 * Timeout values in milliseconds
 */
export const TIMEOUTS = {
  /** Short wait for UI updates */
  SHORT: 500,
  /** Medium wait for page transitions */
  MEDIUM: 1000,
  /** Long wait for complex operations */
  LONG: 3000,
  /** Maximum wait for app loading */
  APP_LOAD: 25000,
  /** Default test timeout */
  TEST: 60000,
  /** Page navigation timeout */
  NAVIGATION: 20000,
  /** API request timeout */
  API: 10000,
} as const;

/**
 * Common CSS selectors used across tests
 */
export const SELECTORS = {
  navigation: {
    mainNav: '#main-navigation',
    sidebar: 'aside nav',
    navRole: 'nav[role="navigation"]',
  },
  common: {
    body: 'body',
    loading: '[data-testid="loading"], [class*="loading"], [class*="spinner"]',
    error: '[role="alert"], [class*="error"]',
    // Empty state selectors - use as array in checkAnyVisible
    empty: '[class*="empty"]',
    emptyText: 'text=/empty/i',
    noData: 'text=/No /i',
  },
  buttons: {
    create: 'button:has-text("Create")',
    add: 'button:has-text("Add")',
    save: 'button:has-text("Save")',
    close: 'button:has-text("Close"), button[aria-label*="close" i]',
    upload: 'button:has-text("Upload")',
    delete: 'button:has-text("Delete")',
    clear: 'button:has-text("Clear")',
  },
  inputs: {
    text: 'input[type="text"]',
    search: 'input[type="search"], input[placeholder*="search" i]',
    filter: 'input[placeholder*="filter" i]',
    password: 'input[type="password"]',
    checkbox: 'input[type="checkbox"]',
  },
  forms: {
    submit: 'button[type="submit"]',
    textarea: 'textarea',
    select: 'select, [role="combobox"]',
  },
  dialogs: {
    modal: '[role="dialog"], [class*="modal"], [class*="dialog"]',
  },
} as const;

