import { logger } from '@/utils/logger';
import React, { useRef, useCallback, useEffect, useState } from 'react';

export interface FocusableElement {
  element: HTMLElement;
  tabIndex?: number;
  disabled?: boolean;
}

export interface UseFocusManagementOptions {
  loop?: boolean; // Whether to loop from last to first element
  autoFocus?: boolean; // Whether to auto-focus first element
  restoreFocus?: boolean; // Whether to restore focus when component unmounts
  trapFocus?: boolean; // Whether to trap focus within the container
}

export function useFocusManagement({
  loop = true,
  autoFocus = false,
  restoreFocus = true,
  trapFocus = false,
}: UseFocusManagementOptions = {}) {
  const containerRef = useRef<HTMLElement>(null);
  const previouslyFocusedElementRef = useRef<HTMLElement | null>(null);
  const [focusedIndex, setFocusedIndex] = useState(-1);

  // Get all focusable elements within the container
  const getFocusableElements = useCallback((): HTMLElement[] => {
    if (!containerRef.current) return [];

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

    const elements = Array.from(
      containerRef.current.querySelectorAll<HTMLElement>(focusableSelectors)
    );

    // Filter out elements that are hidden or have negative tabindex
    return elements.filter(element => {
      const style = window.getComputedStyle(element);
      return (
        style.display !== 'none' &&
        style.visibility !== 'hidden' &&
        !element.hasAttribute('aria-hidden') &&
        element.tabIndex >= 0
      );
    });
  }, []);

  // Focus specific element by index
  const focusElementByIndex = useCallback((index: number) => {
    const elements = getFocusableElements();
    if (elements.length === 0) return false;

    let targetIndex = index;
    
    if (loop) {
      if (targetIndex < 0) {
        targetIndex = elements.length - 1;
      } else if (targetIndex >= elements.length) {
        targetIndex = 0;
      }
    } else {
      targetIndex = Math.max(0, Math.min(targetIndex, elements.length - 1));
    }

    const element = elements[targetIndex];
    if (element) {
      element.focus();
      setFocusedIndex(targetIndex);
      return true;
    }
    return false;
  }, [getFocusableElements, loop]);

  // Focus first focusable element
  const focusFirst = useCallback(() => {
    return focusElementByIndex(0);
  }, [focusElementByIndex]);

  // Focus last focusable element
  const focusLast = useCallback(() => {
    const elements = getFocusableElements();
    return focusElementByIndex(elements.length - 1);
  }, [focusElementByIndex, getFocusableElements]);

  // Focus next focusable element
  const focusNext = useCallback(() => {
    const elements = getFocusableElements();
    const currentIndex = elements.findIndex(element => element === document.activeElement);
    return focusElementByIndex(currentIndex + 1);
  }, [focusElementByIndex, getFocusableElements]);

  // Focus previous focusable element
  const focusPrevious = useCallback(() => {
    const elements = getFocusableElements();
    const currentIndex = elements.findIndex(element => element === document.activeElement);
    return focusElementByIndex(currentIndex - 1);
  }, [focusElementByIndex, getFocusableElements]);

  // Focus specific element
  const focusElement = useCallback((element: HTMLElement) => {
    const elements = getFocusableElements();
    const index = elements.indexOf(element);
    if (index >= 0) {
      return focusElementByIndex(index);
    }
    return false;
  }, [focusElementByIndex, getFocusableElements]);

  // Handle keyboard navigation
  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!containerRef.current) return;

    switch (event.key) {
      case 'ArrowDown':
      case 'ArrowRight':
        event.preventDefault();
        focusNext();
        break;
      case 'ArrowUp':
      case 'ArrowLeft':
        event.preventDefault();
        focusPrevious();
        break;
      case 'Home':
        event.preventDefault();
        focusFirst();
        break;
      case 'End':
        event.preventDefault();
        focusLast();
        break;
      case 'Tab':
        if (trapFocus) {
          event.preventDefault();
          if (event.shiftKey) {
            focusPrevious();
          } else {
            focusNext();
          }
        }
        break;
      case 'Escape':
        if (restoreFocus && previouslyFocusedElementRef.current) {
          previouslyFocusedElementRef.current.focus();
        }
        break;
    }
  }, [focusNext, focusPrevious, focusFirst, focusLast, trapFocus, restoreFocus]);

  // Set up focus management
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // Store previously focused element
    if (restoreFocus) {
      previouslyFocusedElementRef.current = document.activeElement as HTMLElement;
    }

    // Auto-focus first element if requested
    if (autoFocus) {
      focusFirst();
    }

    // Add keyboard event listener
    container.addEventListener('keydown', handleKeyDown);

    return () => {
      container.removeEventListener('keydown', handleKeyDown);
      
      // Restore focus when component unmounts
      if (restoreFocus && previouslyFocusedElementRef.current) {
        previouslyFocusedElementRef.current.focus();
      }
    };
  }, [autoFocus, focusFirst, handleKeyDown, restoreFocus]);

  // Focus trap effect
  useEffect(() => {
    if (!trapFocus) return;

    const handleFocusOut = (event: FocusEvent) => {
      const container = containerRef.current;
      if (!container || !event.relatedTarget) return;

      const isTargetInside = container.contains(event.relatedTarget as Node);
      if (!isTargetInside) {
        // Focus moved outside, bring it back
        focusFirst();
      }
    };

    document.addEventListener('focusout', handleFocusOut);
    return () => document.removeEventListener('focusout', handleFocusOut);
  }, [trapFocus, focusFirst]);

  return {
    containerRef,
    focusFirst,
    focusLast,
    focusNext,
    focusPrevious,
    focusElement,
    focusElementByIndex,
    getFocusableElements,
    focusedIndex,
  };
}

// Hook for managing focus within modals/dialogs
export function useModalFocus() {
  const focus = useFocusManagement({
    autoFocus: true,
    restoreFocus: true,
    trapFocus: true,
    loop: true,
  });

  return focus;
}

// Hook for managing focus within dropdown menus
export function useDropdownFocus() {
  const focus = useFocusManagement({
    autoFocus: true,
    restoreFocus: false,
    trapFocus: false,
    loop: true,
  });

  return focus;
}

// Hook for managing focus within form groups
export function useFormFocus() {
  const focus = useFocusManagement({
    autoFocus: false,
    restoreFocus: false,
    trapFocus: false,
    loop: false,
  });

  return focus;
}

// Hook for skip links functionality
export function useSkipLinks() {
  const skipToContent = useCallback((targetId: string) => {
    const target = document.getElementById(targetId);
    if (target) {
      target.focus();
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }, []);

  const createSkipLink = useCallback((targetId: string, label: string) => {
    return {
      href: `#${targetId}`,
      onClick: (e: React.MouseEvent) => {
        e.preventDefault();
        skipToContent(targetId);
      },
      onKeyDown: (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          skipToContent(targetId);
        }
      },
      children: label,
      className: 'sr-only focus:not-sr-only focus:absolute focus:top-0 focus:left-0 focus:z-50 focus:p-4 focus:bg-background focus:text-foreground focus:border focus:border-border focus:rounded-md',
    };
  }, [skipToContent]);

  return {
    skipToContent,
    createSkipLink,
  };
}

// Utility hook for managing focus announcements
export function useFocusAnnouncement() {
  const [announcement, setAnnouncement] = useState('');
  const announcementRef = useRef<HTMLDivElement>(null);

  const announce = useCallback((message: string, priority: 'polite' | 'assertive' = 'polite') => {
    setAnnouncement(message);
    
    if (announcementRef.current) {
      announcementRef.current.setAttribute('aria-live', priority);
    }

    // Clear announcement after a short delay
    setTimeout(() => setAnnouncement(''), 1000);
  }, []);

  const AnnouncementRegion = useCallback(() => {
    return React.createElement('div', {
      ref: announcementRef,
      'aria-live': 'polite',
      'aria-atomic': 'true',
      className: 'sr-only'
    }, announcement);
  }, [announcement]);

  return {
    announce,
    AnnouncementRegion,
  };
}