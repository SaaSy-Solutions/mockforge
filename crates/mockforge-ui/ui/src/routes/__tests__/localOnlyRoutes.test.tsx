import React, { Suspense } from 'react';
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const cloudMode = vi.hoisted(() => ({ enabled: false }));
vi.mock('../../utils/cloudMode', () => ({
  isCloudMode: () => cloudMode.enabled,
}));
vi.mock('../../pages/SmtpMailboxPage', () => ({
  SmtpMailboxPage: () => <h1>SMTP mailbox page</h1>,
}));
vi.mock('../../pages/PlaygroundPage', () => ({
  PlaygroundPage: () => <h1>Playground page</h1>,
}));
vi.mock('../../pages/IncidentDashboardPage', () => ({
  default: () => <h1>Incident dashboard page</h1>,
  IncidentDashboardPage: () => <h1>Incident dashboard page</h1>,
}));
vi.mock('../../pages/OrganizationPage', () => ({
  OrganizationPage: () => <h1>Organization page</h1>,
}));

import { routes } from '../index';

function PathProbe() {
  return <output data-testid="path">{useLocation().pathname}</output>;
}

function renderPath(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Suspense fallback={<div>Loading</div>}>
        <Routes>
          {routes.map((route) => (
            <Route key={route.path} path={route.path} element={route.element} />
          ))}
        </Routes>
      </Suspense>
      <PathProbe />
    </MemoryRouter>,
  );
}

describe('local-only routes', () => {
  beforeEach(() => {
    cloudMode.enabled = false;
  });

  it('shows a cloud guard for SMTP and the local page outside cloud mode', async () => {
    cloudMode.enabled = true;
    const cloud = renderPath('/smtp-mailbox');
    expect(await screen.findByRole('heading', { name: 'Requires local MockForge' })).toBeVisible();
    expect(screen.getByRole('link', { name: 'Run locally' })).toHaveAttribute(
      'href',
      'https://docs.mockforge.dev/getting-started/installation.html',
    );
    cloud.unmount();

    cloudMode.enabled = false;
    renderPath('/smtp-mailbox');
    expect(await screen.findByRole('heading', { name: 'SMTP mailbox page' })).toBeVisible();
  });

  it('redirects retired user management to organization in both modes', async () => {
    renderPath('/user-management');
    expect(await screen.findByRole('heading', { name: 'Organization page' })).toBeVisible();
    expect(screen.getByTestId('path')).toHaveTextContent('/organization');
  });

  it('guards working local playground and incident pages in cloud mode', async () => {
    cloudMode.enabled = true;
    const playground = renderPath('/playground');
    expect(await screen.findByRole('heading', { name: 'Requires local MockForge' })).toBeVisible();
    playground.unmount();
    const incidents = renderPath('/incident-dashboard');
    expect(await screen.findByRole('heading', { name: 'Requires local MockForge' })).toBeVisible();
  });

  it('renders the working playground page in local mode', async () => {
    cloudMode.enabled = false;
    renderPath('/playground');
    expect(await screen.findByRole('heading', { name: 'Playground page' })).toBeVisible();
  });
});
