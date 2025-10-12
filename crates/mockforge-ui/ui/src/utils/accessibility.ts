import { logger } from '@/utils/logger';
// Utility functions for accessibility

export interface AriaLabelOptions {
  label?: string;
  labelledBy?: string;
  describedBy?: string;
  role?: string;
  expanded?: boolean;
  pressed?: boolean;
  selected?: boolean;
  disabled?: boolean;
  hidden?: boolean;
  live?: 'polite' | 'assertive' | 'off';
  atomic?: boolean;
  relevant?: 'additions' | 'removals' | 'text' | 'all';
}

export function getAriaAttributes(options: AriaLabelOptions) {
  const attrs: Record<string, string | boolean> = {};

  if (options.label) attrs['aria-label'] = options.label;
  if (options.labelledBy) attrs['aria-labelledby'] = options.labelledBy;
  if (options.describedBy) attrs['aria-describedby'] = options.describedBy;
  if (options.role) attrs['role'] = options.role;
  if (typeof options.expanded === 'boolean') attrs['aria-expanded'] = options.expanded;
  if (typeof options.pressed === 'boolean') attrs['aria-pressed'] = options.pressed;
  if (typeof options.selected === 'boolean') attrs['aria-selected'] = options.selected;
  if (typeof options.disabled === 'boolean') attrs['aria-disabled'] = options.disabled;
  if (typeof options.hidden === 'boolean') attrs['aria-hidden'] = options.hidden;
  if (options.live) attrs['aria-live'] = options.live;
  if (typeof options.atomic === 'boolean') attrs['aria-atomic'] = options.atomic;
  if (options.relevant) attrs['aria-relevant'] = options.relevant;

  return attrs;
}

export function generateId(prefix = 'element') {
  return `${prefix}-${Math.random().toString(36).substr(2, 9)}`;
}

// Status announcement utilities
export function announceToScreenReader(message: string, priority: 'polite' | 'assertive' = 'polite') {
  const announcement = document.createElement('div');
  announcement.setAttribute('aria-live', priority);
  announcement.setAttribute('aria-atomic', 'true');
  announcement.className = 'sr-only';
  announcement.textContent = message;

  document.body.appendChild(announcement);

  // Remove after announcement
  setTimeout(() => {
    document.body.removeChild(announcement);
  }, 1000);
}

// Focus management utilities
export function getFocusableElements(container: Element): HTMLElement[] {
  const focusableSelectors = [
    'button:not([disabled])',
    'input:not([disabled])',
    'select:not([disabled])',
    'textarea:not([disabled])',
    'a[href]',
    '[tabindex]:not([tabindex="-1"])',
    '[contenteditable="true"]',
    'summary',
  ].join(', ');

  return Array.from(container.querySelectorAll<HTMLElement>(focusableSelectors))
    .filter(element => {
      const style = window.getComputedStyle(element);
      return (
        style.display !== 'none' &&
        style.visibility !== 'hidden' &&
        !element.hasAttribute('aria-hidden') &&
        element.tabIndex >= 0
      );
    });
}

export function trapFocus(container: Element, event: KeyboardEvent) {
  const focusableElements = getFocusableElements(container);
  const firstElement = focusableElements[0];
  const lastElement = focusableElements[focusableElements.length - 1];

  if (event.key === 'Tab') {
    if (event.shiftKey) {
      if (document.activeElement === firstElement) {
        event.preventDefault();
        lastElement?.focus();
      }
    } else {
      if (document.activeElement === lastElement) {
        event.preventDefault();
        firstElement?.focus();
      }
    }
  }
}

// Color contrast utilities
export function getContrastRatio(color1: string, color2: string): number {
  const rgb1 = hexToRgb(color1);
  const rgb2 = hexToRgb(color2);

  if (!rgb1 || !rgb2) return 0;

  const l1 = getRelativeLuminance(rgb1);
  const l2 = getRelativeLuminance(rgb2);

  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);

  return (lighter + 0.05) / (darker + 0.05);
}

export function meetsWCAGAA(contrastRatio: number, fontSize = 'normal'): boolean {
  // WCAG AA requirements: 4.5:1 for normal text, 3:1 for large text
  const requiredRatio = fontSize === 'large' ? 3 : 4.5;
  return contrastRatio >= requiredRatio;
}

export function meetsWCAGAAA(contrastRatio: number, fontSize = 'normal'): boolean {
  // WCAG AAA requirements: 7:1 for normal text, 4.5:1 for large text
  const requiredRatio = fontSize === 'large' ? 4.5 : 7;
  return contrastRatio >= requiredRatio;
}

function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return result ? {
    r: parseInt(result[1], 16),
    g: parseInt(result[2], 16),
    b: parseInt(result[3], 16)
  } : null;
}

function getRelativeLuminance({ r, g, b }: { r: number; g: number; b: number }): number {
  const rs = r / 255;
  const gs = g / 255;
  const bs = b / 255;

  const rLinear = rs <= 0.03928 ? rs / 12.92 : Math.pow((rs + 0.055) / 1.055, 2.4);
  const gLinear = gs <= 0.03928 ? gs / 12.92 : Math.pow((gs + 0.055) / 1.055, 2.4);
  const bLinear = bs <= 0.03928 ? bs / 12.92 : Math.pow((bs + 0.055) / 1.055, 2.4);

  return 0.2126 * rLinear + 0.7152 * gLinear + 0.0722 * bLinear;
}

// Screen reader detection
export function isScreenReaderActive(): boolean {
  // Check for common screen reader indicators
  return !!(
    window.navigator?.userAgent?.includes('NVDA') ||
    window.navigator?.userAgent?.includes('JAWS') ||
    window.speechSynthesis ||
    (window as Window & { speechSynthesis?: { getVoices?: () => unknown[] } }).speechSynthesis?.getVoices?.()?.length > 0
  );
}

// High contrast mode detection
export function isHighContrastMode(): boolean {
  // Create a test element to check for high contrast mode
  const testElement = document.createElement('div');
  testElement.style.borderWidth = '1px';
  testElement.style.borderStyle = 'solid';
  testElement.style.borderColor = 'rgb(31, 41, 59)'; // A specific color
  testElement.style.position = 'absolute';
  testElement.style.top = '-999px';

  document.body.appendChild(testElement);

  const computedBorderColor = window.getComputedStyle(testElement).borderColor;
  const isHighContrast = computedBorderColor !== 'rgb(31, 41, 59)';

  document.body.removeChild(testElement);

  return isHighContrast;
}

// Reduced motion detection
export function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

// ARIA live region utilities
export class LiveRegionManager {
  private static instance: LiveRegionManager;
  private politeRegion: HTMLElement | null = null;
  private assertiveRegion: HTMLElement | null = null;

  static getInstance(): LiveRegionManager {
    if (!LiveRegionManager.instance) {
      LiveRegionManager.instance = new LiveRegionManager();
    }
    return LiveRegionManager.instance;
  }

  private constructor() {
    this.createLiveRegions();
  }

  private createLiveRegions() {
    // Create polite live region
    this.politeRegion = document.createElement('div');
    this.politeRegion.setAttribute('aria-live', 'polite');
    this.politeRegion.setAttribute('aria-atomic', 'true');
    this.politeRegion.className = 'sr-only';
    this.politeRegion.id = 'live-region-polite';

    // Create assertive live region
    this.assertiveRegion = document.createElement('div');
    this.assertiveRegion.setAttribute('aria-live', 'assertive');
    this.assertiveRegion.setAttribute('aria-atomic', 'true');
    this.assertiveRegion.className = 'sr-only';
    this.assertiveRegion.id = 'live-region-assertive';

    document.body.appendChild(this.politeRegion);
    document.body.appendChild(this.assertiveRegion);
  }

  announce(message: string, priority: 'polite' | 'assertive' = 'polite') {
    const region = priority === 'assertive' ? this.assertiveRegion : this.politeRegion;

    if (region) {
      // Clear first, then set message
      region.textContent = '';
      setTimeout(() => {
        if (region) region.textContent = message;
      }, 10);

      // Clear after announcement
      setTimeout(() => {
        if (region) region.textContent = '';
      }, 1000);
    }
  }
}

// Export singleton instance
export const liveRegion = LiveRegionManager.getInstance();
