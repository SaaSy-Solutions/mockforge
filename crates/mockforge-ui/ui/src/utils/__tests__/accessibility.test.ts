/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import {
  getAriaAttributes,
  generateId,
  announceToScreenReader,
  getFocusableElements,
  trapFocus,
} from '../accessibility';

describe('getAriaAttributes', () => {
  it('returns aria-label when provided', () => {
    const attrs = getAriaAttributes({ label: 'Test Label' });
    expect(attrs['aria-label']).toBe('Test Label');
  });

  it('returns aria-labelledby when provided', () => {
    const attrs = getAriaAttributes({ labelledBy: 'label-id' });
    expect(attrs['aria-labelledby']).toBe('label-id');
  });

  it('returns aria-describedby when provided', () => {
    const attrs = getAriaAttributes({ describedBy: 'desc-id' });
    expect(attrs['aria-describedby']).toBe('desc-id');
  });

  it('returns role when provided', () => {
    const attrs = getAriaAttributes({ role: 'button' });
    expect(attrs['role']).toBe('button');
  });

  it('returns aria-expanded when provided', () => {
    const attrs = getAriaAttributes({ expanded: true });
    expect(attrs['aria-expanded']).toBe(true);
  });

  it('returns aria-pressed when provided', () => {
    const attrs = getAriaAttributes({ pressed: false });
    expect(attrs['aria-pressed']).toBe(false);
  });

  it('returns aria-selected when provided', () => {
    const attrs = getAriaAttributes({ selected: true });
    expect(attrs['aria-selected']).toBe(true);
  });

  it('returns aria-disabled when provided', () => {
    const attrs = getAriaAttributes({ disabled: true });
    expect(attrs['aria-disabled']).toBe(true);
  });

  it('returns aria-hidden when provided', () => {
    const attrs = getAriaAttributes({ hidden: false });
    expect(attrs['aria-hidden']).toBe(false);
  });

  it('returns aria-live when provided', () => {
    const attrs = getAriaAttributes({ live: 'polite' });
    expect(attrs['aria-live']).toBe('polite');
  });

  it('returns aria-atomic when provided', () => {
    const attrs = getAriaAttributes({ atomic: true });
    expect(attrs['aria-atomic']).toBe(true);
  });

  it('returns aria-relevant when provided', () => {
    const attrs = getAriaAttributes({ relevant: 'additions' });
    expect(attrs['aria-relevant']).toBe('additions');
  });

  it('returns empty object when no options provided', () => {
    const attrs = getAriaAttributes({});
    expect(Object.keys(attrs)).toHaveLength(0);
  });

  it('combines multiple attributes', () => {
    const attrs = getAriaAttributes({
      label: 'Button',
      role: 'button',
      expanded: false,
      disabled: true,
    });
    expect(attrs['aria-label']).toBe('Button');
    expect(attrs['role']).toBe('button');
    expect(attrs['aria-expanded']).toBe(false);
    expect(attrs['aria-disabled']).toBe(true);
  });
});

describe('generateId', () => {
  it('generates unique IDs', () => {
    const id1 = generateId();
    const id2 = generateId();
    expect(id1).not.toBe(id2);
  });

  it('uses provided prefix', () => {
    const id = generateId('button');
    expect(id).toMatch(/^button-/);
  });

  it('uses default prefix when not provided', () => {
    const id = generateId();
    expect(id).toMatch(/^element-/);
  });

  it('generates alphanumeric IDs', () => {
    const id = generateId('test');
    expect(id).toMatch(/^test-[a-z0-9]+$/);
  });
});

describe('announceToScreenReader', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('creates announcement element with message', () => {
    announceToScreenReader('Test message');

    const announcement = document.body.querySelector('[aria-live]');
    expect(announcement).toBeTruthy();
    expect(announcement?.textContent).toBe('Test message');
  });

  it('uses polite priority by default', () => {
    announceToScreenReader('Test message');

    const announcement = document.body.querySelector('[aria-live]');
    expect(announcement?.getAttribute('aria-live')).toBe('polite');
  });

  it('uses assertive priority when specified', () => {
    announceToScreenReader('Urgent message', 'assertive');

    const announcement = document.body.querySelector('[aria-live]');
    expect(announcement?.getAttribute('aria-live')).toBe('assertive');
  });

  it('sets aria-atomic attribute', () => {
    announceToScreenReader('Test message');

    const announcement = document.body.querySelector('[aria-live]');
    expect(announcement?.getAttribute('aria-atomic')).toBe('true');
  });

  it('removes announcement after timeout', () => {
    announceToScreenReader('Test message');

    let announcement = document.body.querySelector('[aria-live]');
    expect(announcement).toBeTruthy();

    vi.advanceTimersByTime(1000);

    announcement = document.body.querySelector('[aria-live]');
    expect(announcement).toBeNull();
  });
});

describe('getFocusableElements', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  it('finds focusable buttons', () => {
    document.body.innerHTML = '<button>Click me</button>';
    const elements = getFocusableElements(document.body);
    expect(elements).toHaveLength(1);
    expect(elements[0].tagName).toBe('BUTTON');
  });

  it('excludes disabled buttons', () => {
    document.body.innerHTML = '<button disabled>Click me</button>';
    const elements = getFocusableElements(document.body);
    expect(elements).toHaveLength(0);
  });

  it('finds focusable inputs', () => {
    document.body.innerHTML = '<input type="text" />';
    const elements = getFocusableElements(document.body);
    expect(elements).toHaveLength(1);
    expect(elements[0].tagName).toBe('INPUT');
  });

  it('finds links with href', () => {
    document.body.innerHTML = '<a href="#test">Link</a>';
    const elements = getFocusableElements(document.body);
    expect(elements).toHaveLength(1);
    expect(elements[0].tagName).toBe('A');
  });

  it('excludes links without href', () => {
    document.body.innerHTML = '<a>Not a link</a>';
    const elements = getFocusableElements(document.body);
    expect(elements).toHaveLength(0);
  });

  it('finds elements with tabindex', () => {
    document.body.innerHTML = '<div tabindex="0">Focusable div</div>';
    const elements = getFocusableElements(document.body);
    expect(elements).toHaveLength(1);
  });

  it('excludes elements with tabindex=-1', () => {
    document.body.innerHTML = '<div tabindex="-1">Non-focusable div</div>';
    const elements = getFocusableElements(document.body);
    expect(elements).toHaveLength(0);
  });

  it('excludes hidden elements', () => {
    document.body.innerHTML = '<button style="display: none">Hidden</button>';
    const elements = getFocusableElements(document.body);
    expect(elements).toHaveLength(0);
  });

  it('excludes elements with aria-hidden', () => {
    document.body.innerHTML = '<button aria-hidden="true">Hidden</button>';
    const elements = getFocusableElements(document.body);
    expect(elements).toHaveLength(0);
  });

  it('finds multiple focusable elements', () => {
    document.body.innerHTML = `
      <button>Button 1</button>
      <input type="text" />
      <a href="#link">Link</a>
      <button>Button 2</button>
    `;
    const elements = getFocusableElements(document.body);
    expect(elements).toHaveLength(4);
  });
});

describe('trapFocus', () => {
  let container: HTMLDivElement;
  let firstButton: HTMLButtonElement;
  let middleButton: HTMLButtonElement;
  let lastButton: HTMLButtonElement;

  beforeEach(() => {
    container = document.createElement('div');
    firstButton = document.createElement('button');
    middleButton = document.createElement('button');
    lastButton = document.createElement('button');

    firstButton.textContent = 'First';
    middleButton.textContent = 'Middle';
    lastButton.textContent = 'Last';

    container.appendChild(firstButton);
    container.appendChild(middleButton);
    container.appendChild(lastButton);

    document.body.appendChild(container);
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  it('focuses last element when Tab on first element with Shift', () => {
    firstButton.focus();

    const event = new KeyboardEvent('keydown', {
      key: 'Tab',
      shiftKey: true,
      bubbles: true,
    });

    Object.defineProperty(event, 'preventDefault', {
      value: vi.fn(),
    });

    trapFocus(container, event as any);

    expect(document.activeElement).toBe(lastButton);
  });

  it('focuses first element when Tab on last element without Shift', () => {
    lastButton.focus();

    const event = new KeyboardEvent('keydown', {
      key: 'Tab',
      shiftKey: false,
      bubbles: true,
    });

    Object.defineProperty(event, 'preventDefault', {
      value: vi.fn(),
    });

    trapFocus(container, event as any);

    expect(document.activeElement).toBe(firstButton);
  });

  it('does not trap focus on non-Tab keys', () => {
    firstButton.focus();

    const event = new KeyboardEvent('keydown', {
      key: 'Enter',
      bubbles: true,
    });

    trapFocus(container, event as any);

    expect(document.activeElement).toBe(firstButton);
  });
});
