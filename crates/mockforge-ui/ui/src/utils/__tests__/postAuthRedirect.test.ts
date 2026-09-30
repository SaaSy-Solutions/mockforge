import { describe, expect, it } from 'vitest';
import { resolvePostAuthTarget, safeRedirectTarget } from '../postAuthRedirect';

describe('safeRedirectTarget', () => {
  it.each(['/billing', '/billing?interval=year', '/workspaces/abc#x'])('accepts app path %s', (p) => {
    expect(safeRedirectTarget(p)).toBe(p);
  });

  it.each([
    null,
    '',
    'billing',
    'https://evil.example/',
    '//evil.example',
    '/\\evil.example',
    '/login',
    '/login?redirect=/billing',
    '/register',
    '/signup',
  ])('rejects %s', (p) => {
    expect(safeRedirectTarget(p)).toBeNull();
  });
});

describe('resolvePostAuthTarget', () => {
  it('prefers an explicit redirect', () => {
    expect(resolvePostAuthTarget(new URLSearchParams('redirect=/billing&plan=team'), '/dashboard')).toBe(
      '/billing',
    );
  });

  it('sends paid-plan sign-ups to billing', () => {
    expect(resolvePostAuthTarget(new URLSearchParams('plan=team'), '/dashboard')).toBe('/billing');
  });

  it('falls back to the default page', () => {
    expect(resolvePostAuthTarget(new URLSearchParams('plan=bogus'), '/dashboard')).toBe('/dashboard');
    expect(resolvePostAuthTarget(new URLSearchParams(''), '/workspaces')).toBe('/workspaces');
  });
});
