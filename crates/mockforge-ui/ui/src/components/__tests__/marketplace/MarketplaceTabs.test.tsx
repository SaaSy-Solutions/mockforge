/**
 * @jest-environment jsdom
 */

import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { MemoryRouter, Routes, Route, useLocation } from 'react-router-dom';
import { MarketplaceTabs } from '../../marketplace/MarketplaceTabs';

function LocationReadout() {
  const loc = useLocation();
  return <div data-testid="pathname">{loc.pathname}</div>;
}

function renderWithRoute(initialPath: string) {
  return render(
    <MemoryRouter initialEntries={[initialPath]}>
      <MarketplaceTabs />
      <Routes>
        <Route path="*" element={<LocationReadout />} />
      </Routes>
    </MemoryRouter>
  );
}

describe('MarketplaceTabs', () => {
  it('renders Templates, Scenarios, and Plugins tabs', () => {
    renderWithRoute('/template-marketplace');
    expect(screen.getByRole('link', { name: /templates/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /scenarios/i })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /plugins/i })).toBeInTheDocument();
  });

  it('highlights the active tab based on current pathname', () => {
    renderWithRoute('/scenario-marketplace');
    const scenariosTab = screen.getByRole('link', { name: /scenarios/i });
    expect(scenariosTab).toHaveAttribute('aria-current', 'page');

    const templatesTab = screen.getByRole('link', { name: /templates/i });
    expect(templatesTab).not.toHaveAttribute('aria-current');
  });

  it('navigates to the tab target when clicked', () => {
    renderWithRoute('/template-marketplace');
    expect(screen.getByTestId('pathname')).toHaveTextContent('/template-marketplace');

    fireEvent.click(screen.getByRole('link', { name: /plugins/i }));
    expect(screen.getByTestId('pathname')).toHaveTextContent('/plugin-registry');
  });

  it('does not claim an unrelated route is the current marketplace page', () => {
    renderWithRoute('/something-else');
    const templatesTab = screen.getByRole('link', { name: /templates/i });
    expect(templatesTab).not.toHaveAttribute('aria-current');
  });
});
