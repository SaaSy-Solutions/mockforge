import { describe, it, expect } from 'vitest';
import { cn } from '../cn';

describe('cn utility', () => {
  it('merges class names correctly', () => {
    const result = cn('class1', 'class2');
    expect(result).toContain('class1');
    expect(result).toContain('class2');
  });

  it('handles conditional classes', () => {
    const isActive = true;
    const result = cn('base', isActive && 'active');
    expect(result).toContain('base');
    expect(result).toContain('active');
  });

  it('filters out false values', () => {
    const result = cn('class1', false, 'class2', null, undefined);
    expect(result).toContain('class1');
    expect(result).toContain('class2');
    expect(result).not.toContain('false');
    expect(result).not.toContain('null');
  });

  it('merges Tailwind conflicting classes correctly', () => {
    const result = cn('px-2', 'px-4');
    // tailwind-merge should keep only px-4
    expect(result).toBe('px-4');
  });

  it('handles arrays of classes', () => {
    const result = cn(['class1', 'class2']);
    expect(result).toContain('class1');
    expect(result).toContain('class2');
  });

  it('handles objects with boolean values', () => {
    const result = cn({
      active: true,
      disabled: false,
      visible: true,
    });
    expect(result).toContain('active');
    expect(result).toContain('visible');
    expect(result).not.toContain('disabled');
  });

  it('returns empty string for no arguments', () => {
    const result = cn();
    expect(result).toBe('');
  });
});
