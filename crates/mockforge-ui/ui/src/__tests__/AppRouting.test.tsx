import React from 'react';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { MemoryRouter, useLocation, useNavigate } from 'react-router-dom';
import { beforeAll, beforeEach, afterEach, describe, expect, it, vi } from 'vitest';

const lazyGates = vi.hoisted(() => new Map<string, { ready: boolean; promise: Promise<void>; resolve: () => void }>());
function DelayedPage({ name }: { name: string }) {
  const gate = lazyGates.get(name);
  if (gate && !gate.ready) throw gate.promise;
  return <h1>{name}</h1>;
}

// --- Hermetic stand-ins for everything App wires up around its routes ---

const authState = {
  isAuthenticated: false,
  user: null as null | { id: string; username: string; role: 'admin' },
  isLoading: false,
  checkAuth: vi.fn().mockResolvedValue(undefined),
};

vi.mock('../stores/useAuthStore', () => {
  const useAuthStore = (selector?: (s: typeof authState) => unknown) =>
    selector ? selector(authState) : authState;
  useAuthStore.getState = () => authState;
  return { useAuthStore };
});

vi.mock('../stores/useWorkspaceStore', () => {
  const state = { loadWorkspaces: vi.fn() };
  return { useWorkspaceStore: (selector: (s: typeof state) => unknown) => selector(state) };
});

vi.mock('../stores/usePreferencesStore', () => {
  const state = { loadPreferences: vi.fn(), preferences: { ui: { defaultPage: '' } } };
  return { usePreferencesStore: (selector: (s: typeof state) => unknown) => selector(state) };
});

vi.mock('../hooks/usePrefetch', () => ({ useStartupPrefetch: vi.fn() }));
vi.mock('../hooks/useThemeSync', () => ({ useThemeSync: vi.fn() }));
vi.mock('../i18n/I18nProvider', () => ({ useI18n: () => ({ t: (key: string) => key }) }));
vi.mock('../components/layout/AppShell', () => ({
  AppShell: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));
vi.mock('../components/auth/LoginForm', () => ({
  LoginForm: () => <div>login form</div>,
}));
vi.mock('../routes', () => ({
  routes: [
    { path: '/dashboard', element: <div>dashboard page</div> },
    { path: '/billing', element: <div>billing page</div> },
    { path: '/state-machines', element: <h1>State Machines</h1> },
    ...['orchestration-builder', 'graph', 'observability'].map(name => ({ path: `/${name}`, element: <DelayedPage name={name} /> })),
  ],
}));

import App from '../App';
import { authApi } from '../services/authApi';

function LocationProbe() {
  const location = useLocation();
  const navigate = useNavigate();
  return <><button onClick={() => React.startTransition(() => navigate('/orchestration-builder'))}>Open orchestration</button><button onClick={() => React.startTransition(() => navigate('/graph'))}>Open graph</button><button onClick={() => React.startTransition(() => navigate('/observability'))}>Open observability</button> <div data-testid="location">{location.pathname + location.search}</div></>;
}

function renderAt(url: string) {
  return render(
    <MemoryRouter initialEntries={[url]}>
      <App />
      <LocationProbe />
    </MemoryRouter>,
  );
}

function signIn() {
  authState.isAuthenticated = true;
  authState.user = { id: '1', username: 'ray', role: 'admin' };
}

describe('App routing', () => {
  afterEach(() => { if (vi.isMockFunction(authApi.resetPassword)) vi.mocked(authApi.resetPassword).mockRestore(); });
  beforeEach(() => {
    authState.isAuthenticated = false;
    authState.user = null;
    authState.checkAuth.mockClear();
  });

  it('replaces the previous editor during delayed sidebar route loading', async () => {
    signIn();
    for (const name of ['orchestration-builder', 'graph', 'observability']) {
      let resolve!: () => void;
      const promise = new Promise<void>(r => { resolve = r; });
      lazyGates.set(name, { ready: false, promise, resolve });
    }
    renderAt('/state-machines');
    expect(await screen.findByRole('heading', { name: 'State Machines' })).toBeVisible();
    for (const [name, button] of [['orchestration-builder', 'Open orchestration'], ['graph', 'Open graph'], ['observability', 'Open observability']]) {
      fireEvent.click(screen.getByRole('button', { name: button }));
      expect(await screen.findByText('app.loading')).toBeVisible();
      expect(screen.queryByRole('heading', { name: 'State Machines' })).not.toBeInTheDocument();
      expect(screen.getByTestId('location')).toHaveTextContent(`/${name}`);
      await act(async () => { const gate = lazyGates.get(name)!; gate.ready = true; gate.resolve(); await gate.promise; });
      expect(await screen.findByRole('heading', { name })).toBeVisible();
    }
    lazyGates.clear();
  });

  it('opens recovery publicly and clears expired reset feedback when requesting a new link', async () => {
    vi.spyOn(authApi, 'resetPassword').mockRejectedValue(new Error('Expired reset link'));
    renderAt('/reset-password?token=expired-fixture');
    fireEvent.change(screen.getByLabelText('New password'), { target: { value: 'Fixture-password' } });
    fireEvent.change(screen.getByLabelText('Confirm password'), { target: { value: 'Fixture-password' } });
    fireEvent.click(screen.getByRole('button', { name: 'Reset password' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Expired reset link');
    fireEvent.click(screen.getByRole('link', { name: 'Request a new reset link' }));
    expect(await screen.findByLabelText('Email')).toBeVisible();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(authState.checkAuth).not.toHaveBeenCalled();
  });

  describe('auth entry paths', () => {
    it('shows the login form at /login while signed out', async () => {
      renderAt('/login');
      expect(await screen.findByText('login form')).toBeInTheDocument();
    });

    it('forwards a signed-in user on /login to the dashboard instead of Not Found', async () => {
      signIn();
      renderAt('/login');
      expect(await screen.findByText('dashboard page')).toBeInTheDocument();
      expect(screen.getByTestId('location')).toHaveTextContent('/dashboard');
      expect(screen.queryByText('app.pageNotFoundTitle')).not.toBeInTheDocument();
    });

    it('honours ?redirect= after sign-in', async () => {
      signIn();
      renderAt('/login?redirect=%2Fbilling%3Finterval%3Dyear');
      expect(await screen.findByText('billing page')).toBeInTheDocument();
      expect(screen.getByTestId('location')).toHaveTextContent('/billing?interval=year');
    });

    it('ignores an off-site ?redirect=', async () => {
      signIn();
      renderAt('/login?redirect=%2F%2Fevil.example');
      expect(await screen.findByText('dashboard page')).toBeInTheDocument();
    });

    it('lands /register?plan=pro on billing after sign-up', async () => {
      signIn();
      renderAt('/register?plan=pro');
      expect(await screen.findByText('billing page')).toBeInTheDocument();
    });
  });

  describe('legal pages', () => {
    // Warm the lazily loaded chunk so findBy* timing doesn't depend on a cold import.
    beforeAll(async () => {
      await import('../pages/LegalDocumentPage');
    });

    it.each([
      ['/legal/privacy', 'Privacy Policy'],
      ['/legal/terms', 'Terms of Service'],
      ['/legal/dpa', 'Data Processing Agreement'],
    ])('renders %s without signing in', async (url, title) => {
      renderAt(url);
      expect(await screen.findByRole('heading', { level: 1, name: title }, { timeout: 5000 })).toBeInTheDocument();
      expect(screen.queryByText('login form')).not.toBeInTheDocument();
      expect(authState.checkAuth).not.toHaveBeenCalled();
    });

    it('renders legal pages for signed-in users too (no Not Found)', async () => {
      signIn();
      renderAt('/legal/privacy');
      expect(
        await screen.findByRole('heading', { level: 1, name: 'Privacy Policy' }, { timeout: 5000 }),
      ).toBeInTheDocument();
    });

    it.each(['terms', 'privacy', 'dpa'])('redirects legacy /%s to /legal/%s', async (id) => {
      renderAt(`/${id}`);
      await screen.findByRole('heading', { level: 1 }, { timeout: 5000 });
      expect(screen.getByTestId('location')).toHaveTextContent(`/legal/${id}`);
    });
  });
});
