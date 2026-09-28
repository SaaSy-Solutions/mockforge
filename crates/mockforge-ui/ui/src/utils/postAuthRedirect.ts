/**
 * Auth entry points (`/login`, `/signup`, `/register`). While the visitor is
 * unauthenticated, AuthGuard renders the LoginForm in place of any route, so
 * these paths only reach the router once auth has flipped to true. At that
 * point they must forward to the page the user was headed to instead of
 * falling through to the catch-all "Page Not Found".
 */
export const AUTH_ENTRY_PATHS = ['/login', '/signup', '/register'] as const;

/**
 * Accept only same-origin, absolute app paths. Rejects protocol-relative
 * (`//evil.com`), backslash tricks (`/\evil.com`), absolute URLs, and loops
 * back into another auth entry path.
 */
export function safeRedirectTarget(raw: string | null): string | null {
  if (!raw) return null;
  if (!raw.startsWith('/') || raw.startsWith('//') || raw.startsWith('/\\')) return null;
  const pathname = raw.split(/[?#]/, 1)[0];
  if ((AUTH_ENTRY_PATHS as readonly string[]).includes(pathname)) return null;
  return raw;
}

/** Where to send a freshly authenticated user, given the auth-entry query string. */
export function resolvePostAuthTarget(params: URLSearchParams, fallback: string): string {
  const explicit = safeRedirectTarget(params.get('redirect'));
  if (explicit) return explicit;
  // Marketing CTAs link to /register?plan=pro|team; land those on billing.
  const plan = params.get('plan');
  if (plan === 'pro' || plan === 'team') return '/billing';
  return fallback;
}
