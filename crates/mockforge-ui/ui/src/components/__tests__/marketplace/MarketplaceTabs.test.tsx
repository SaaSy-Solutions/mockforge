/**
 * @jest-environment jsdom
 */

import React from 'react';
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
    expect(screen.getByRole('tab', { name: /templates/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /scenarios/i })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /plugins/i })).toBeInTheDocument();
  });

  it('highlights the active tab based on current pathname', () => {
    renderWithRoute('/scenario-marketplace');
    const scenariosTab = screen.getByRole('tab', { name: /scenarios/i });
    expect(scenariosTab).toHaveAttribute('aria-selected', 'true');

    const templatesTab = screen.getByRole('tab', { name: /templates/i });
    expect(templatesTab).toHaveAttribute('aria-selected', 'false');
  });

  it('navigates to the tab target when clicked', () => {
    renderWithRoute('/template-marketplace');
    expect(screen.getByTestId('pathname')).toHaveTextContent('/template-marketplace');

    fireEvent.click(screen.getByRole('tab', { name: /plugins/i }));
    expect(screen.getByTestId('pathname')).toHaveTextContent('/plugin-registry');
  });

  it('falls back to Templates tab when pathname is unknown', () => {
    renderWithRoute('/something-else');
    const templatesTab = screen.getByRole('tab', { name: /templates/i });
    expect(templatesTab).toHaveAttribute('aria-selected', 'true');
  });
});
