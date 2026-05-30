import React, { useEffect, useRef } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { useAuthStore } from '@/stores/useAuthStore';
import { authApi } from '@/services/authApi';
import { logger } from '@/utils/logger';

// Error codes returned by the backend as sso_error= query parameter.
const SSO_ERROR_MESSAGES: Record<string, string> = {
  invalid_state: 'SSO state parameter was invalid. Please try again.',
  callback_failed: 'SSO authentication failed. Please try again.',
  user_not_found: 'Your SSO account could not be matched to a user.',
  sso_disabled: 'SSO is not enabled for your organization.',
  domain_not_verified: 'The email domain for your organization has not been verified.',
  internal_error: 'An internal error occurred during SSO login.',
};

/**
 * SsoCallbackPage — landing page after an IdP SSO round-trip.
 *
 * Success path: backend 302s to /auth/sso/callback?token=<jwt>&org_slug=<slug>
 * Failure path: backend 302s to /login?sso_error=<code>
 *
 * On success we persist the token via the auth store's setAuthenticated action
 * (same path used by manual registration), hydrate /users/me, then redirect
 * into the app. This ensures auto-refresh, axios/fetch auth headers, and the
 * localStorage sync subscriber all fire exactly as they do on a normal login.
 *
 * This route MUST be listed in AuthGuard's PUBLIC_PREFIXES so the user
 * can land here before they are authenticated.
 */
export function SsoCallbackPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { setAuthenticated } = useAuthStore();
  // Guard against React 18 StrictMode double-invocation in dev.
  const handled = useRef(false);

  useEffect(() => {
    if (handled.current) return;
    handled.current = true;

    const token = searchParams.get('token');
    // org_slug is available for analytics / logging but not needed to complete auth.
    const orgSlug = searchParams.get('org_slug');
    const ssoError = searchParams.get('sso_error');

    if (ssoError) {
      // Backend redirected here via /login?sso_error=<code> in some flows, but
      // we also handle it if it appears on the callback URL directly.
      // Navigate to root — the AuthGuard will show the LoginForm because the user
      // is unauthenticated, and the LoginForm reads the sso_error query param.
      const message = SSO_ERROR_MESSAGES[ssoError] ?? `SSO error: ${ssoError}`;
      navigate(`/?sso_error=${encodeURIComponent(message)}`, { replace: true });
      return;
    }

    if (!token) {
      logger.warn('SsoCallbackPage: no token in URL');
      navigate('/login?sso_error=' + encodeURIComponent('No authentication token received.'), {
        replace: true,
      });
      return;
    }

    // Persist the token immediately so subsequent API calls carry the
    // Authorization header (same pattern as register in LoginForm).
    // We build a minimal User from the JWT payload; hydrateUserFromServer
    // (called inside checkAuth / the store internals) will fill in the rest.
    const minimalUser = {
      id: '',
      username: '',
      email: '',
      role: 'user' as const,
    };
    setAuthenticated(minimalUser, token);

    // Hydrate the full profile via /users/me, then navigate into the app.
    authApi
      .getMe()
      .then((profile) => {
        // Re-call setAuthenticated with real user data so the store has the
        // full profile (role, email, username) before we navigate.
        setAuthenticated(
          {
            id: profile.user_id,
            username: profile.username,
            email: profile.email,
            role: profile.is_admin ? 'admin' : 'user',
            is_verified: profile.is_verified,
            created_at: profile.created_at,
          },
          token,
        );
        logger.info('SSO login successful', { orgSlug, userId: profile.user_id });
        navigate('/', { replace: true });
      })
      .catch((err) => {
        logger.error('SsoCallbackPage: /users/me hydration failed', err);
        // Token was accepted and stored; still navigate into the app — the
        // auto-refresh flow will fill in the user profile on next checkAuth.
        navigate('/', { replace: true });
      });
  }, [searchParams, navigate, setAuthenticated]);

  return (
    <div className="min-h-screen flex items-center justify-center bg-background">
      <div className="text-center space-y-4">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto" />
        <p className="text-muted-foreground">Signing you in&hellip;</p>
      </div>
    </div>
  );
}
