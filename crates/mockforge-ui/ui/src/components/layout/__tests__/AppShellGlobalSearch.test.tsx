import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';

vi.mock('../ConnectionStatus', () => ({ GlobalConnectionStatus: () => null }));
vi.mock('../../auth/UserProfile', () => ({ UserProfile: () => null }));
vi.mock('../../auth/HelpSupport', () => ({ HelpSupport: () => null }));

import { AppShell } from '../AppShell';
import { I18nProvider } from '../../../i18n/I18nProvider';

function LocationProbe() {
  return <div data-testid="location">{useLocation().pathname}</div>;
}

function renderShell() {
  render(
    <I18nProvider>
      <MemoryRouter initialEntries={['/dashboard']}>
        <AppShell onRefresh={() => {}}>
          <div />
        </AppShell>
        <LocationProbe />
      </MemoryRouter>
    </I18nProvider>,
  );
  return document.getElementById('global-search-input') as HTMLInputElement;
}

describe('AppShell global search', () => {
  it('suggests pages and navigates to the first match on Enter', () => {
    const input = renderShell();
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: 'billing' } });

    expect(screen.getByRole('listbox', { name: 'Pages' })).toBeInTheDocument();
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(screen.getByTestId('location')).toHaveTextContent('/billing');
    expect(input.value).toBe('');
  });

  it('finds pages by concept keyword, e.g. "override"', () => {
    const input = renderShell();
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: 'override' } });

    const options = screen.getAllByRole('option').map((o) => o.id);
    expect(options).toEqual(
      expect.arrayContaining(['global-search-page-fixtures', 'global-search-page-config']),
    );
  });

  it('says so when nothing matches', () => {
    const input = renderShell();
    fireEvent.focus(input);
    fireEvent.change(input, { target: { value: 'zzqqxx' } });

    expect(screen.getByText(/No pages match/)).toBeInTheDocument();
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(screen.getByTestId('location')).toHaveTextContent('/dashboard');
  });
});
