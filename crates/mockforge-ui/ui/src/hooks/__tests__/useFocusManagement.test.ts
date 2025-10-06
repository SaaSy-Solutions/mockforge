/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useFocusManagement } from '../useFocusManagement';

describe('useFocusManagement', () => {
  let container: HTMLDivElement;
  let button1: HTMLButtonElement;
  let button2: HTMLButtonElement;
  let button3: HTMLButtonElement;

  beforeEach(() => {
    container = document.createElement('div');
    button1 = document.createElement('button');
    button2 = document.createElement('button');
    button3 = document.createElement('button');

    button1.textContent = 'Button 1';
    button2.textContent = 'Button 2';
    button3.textContent = 'Button 3';

    container.appendChild(button1);
    container.appendChild(button2);
    container.appendChild(button3);

    document.body.appendChild(container);
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  it('focuses first element', () => {
    const { result } = renderHook(() => useFocusManagement());

    act(() => {
      result.current.containerRef.current = container;
      result.current.focusFirst();
    });

    expect(document.activeElement).toBe(button1);
  });

  it('focuses last element', () => {
    const { result } = renderHook(() => useFocusManagement());

    act(() => {
      result.current.containerRef.current = container;
      result.current.focusLast();
    });

    expect(document.activeElement).toBe(button3);
  });

  it('focuses next element', () => {
    const { result } = renderHook(() => useFocusManagement());

    act(() => {
      result.current.containerRef.current = container;
      button1.focus();
      result.current.focusNext();
    });

    expect(document.activeElement).toBe(button2);
  });

  it('focuses previous element', () => {
    const { result } = renderHook(() => useFocusManagement());

    act(() => {
      result.current.containerRef.current = container;
      button2.focus();
      result.current.focusPrevious();
    });

    expect(document.activeElement).toBe(button1);
  });

  it('loops from last to first when enabled', () => {
    const { result } = renderHook(() => useFocusManagement({ loop: true }));

    act(() => {
      result.current.containerRef.current = container;
      button3.focus();
      result.current.focusNext();
    });

    expect(document.activeElement).toBe(button1);
  });

  it('does not loop when disabled', () => {
    const { result } = renderHook(() => useFocusManagement({ loop: false }));

    act(() => {
      result.current.containerRef.current = container;
      button3.focus();
      result.current.focusNext();
    });

    // Should stay on button3
    expect(document.activeElement).toBe(button3);
  });
});
