import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../ConnectionStatus', () => ({ GlobalConnectionStatus: () => null }));
vi.mock('../../auth/UserProfile', () => ({ UserProfile: () => null }));
vi.mock('../../auth/HelpSupport', () => ({ HelpSupport: () => null }));

import { AppShell } from '../AppShell';
import { I18nProvider } from '../../../i18n/I18nProvider';

function LocationProbe() {
  return <div data-testid="location">{useLocation().pathname}</div>;
}

function renderShell(path = '/dashboard', onRefresh?: () => void) {
  render(
    <I18nProvider>
      <MemoryRouter initialEntries={[path]}>
        <AppShell onRefresh={onRefresh}>
          <div />
        </AppShell>
        <LocationProbe />
      </MemoryRouter>
    </I18nProvider>,
  );
}

function openPalette() {
  fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
  return document.getElementById('global-search-input') as HTMLInputElement;
}

describe('AppShell command palette', () => {
  beforeEach(() => localStorage.clear());

  it('opens with Ctrl+K and navigates to the first page match on Enter', () => {
    renderShell();
    const input = openPalette();
    expect(input).toBeInTheDocument();

    fireEvent.change(input, { target: { value: 'billing' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(screen.getByTestId('location')).toHaveTextContent('/billing');
    expect(document.getElementById('global-search-input')).toBeNull();
  });

  it('also opens from the sidebar search button', () => {
    renderShell();
    fireEvent.click(screen.getAllByRole('button', { name: 'Search' })[0]);
    expect(document.getElementById('global-search-input')).toBeInTheDocument();
  });

  it('finds pages by concept keyword, e.g. "override"', () => {
    renderShell();
    const input = openPalette();
    fireEvent.change(input, { target: { value: 'override' } });

    const pages = screen
      .getAllByRole('option')
      .map((o) => o.id)
      .filter((id) => id.startsWith('cmd-page:'));
    expect(pages[0]).toBe('cmd-page:overrides');
  });

  it('offers log and service search for free text', () => {
    renderShell();
    const input = openPalette();
    fireEvent.change(input, { target: { value: '/api/users' } });

    const list = screen.getByRole('listbox');
    expect(within(list).getByText(/Search logs for/)).toBeInTheDocument();
    expect(within(list).getByText(/Search services for/)).toBeInTheDocument();
  });

  it('offers no pages when nothing matches, only free-text search', () => {
    renderShell();
    const input = openPalette();
    fireEvent.change(input, { target: { value: 'zzqqxx' } });

    const ids = screen.getAllByRole('option').map((o) => o.id);
    expect(ids).toEqual(['cmd-action:logs', 'cmd-action:services']);
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(screen.getByTestId('location')).toHaveTextContent('/dashboard');
  });
});

describe('AppShell sidebar', () => {
  beforeEach(() => localStorage.clear());

  it('does not duplicate page headings or expose a refresh button without a callback', () => {
    renderShell('/cloud-traces');
    expect(screen.queryByRole('heading')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Refresh' })).not.toBeInTheDocument();
    const input = openPalette();
    fireEvent.change(input, { target: { value: 'refresh' } });
    expect(screen.queryByRole('option', { name: /Refresh current view/ })).not.toBeInTheDocument();
  });

  it('highlights the current page and shows it in the breadcrumb', () => {
    renderShell('/logs');
    const link = screen.getByRole('link', { name: 'Logs' });
    expect(link).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('navigation', { name: 'Breadcrumb' })).toHaveTextContent('Observe');
  });

  it('keeps folded sections folded until opened, and opens the active one', () => {
    renderShell('/billing');
    // Settings contains the active page, so it is unfolded automatically.
    expect(screen.getByRole('link', { name: 'Billing' })).toBeInTheDocument();
    // Flows is folded by default.
    expect(screen.queryByRole('link', { name: 'Chains' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: /Flows/ }));
    expect(screen.getByRole('link', { name: 'Chains' })).toBeInTheDocument();
  });
});
