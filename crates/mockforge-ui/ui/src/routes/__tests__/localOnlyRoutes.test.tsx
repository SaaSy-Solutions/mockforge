import { Suspense } from 'react';
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
vi.mock('../../pages/CloudIncidentsPage', () => ({ default: () => <h1>Cloud incidents page</h1>, CloudIncidentsPage: () => <h1>Cloud incidents page</h1> }));
vi.mock('../../pages/OrganizationPage', () => ({
  OrganizationPage: () => <h1>Organization page</h1>,
}));
vi.mock('../../pages/AnalyticsPage', () => ({ default: () => <h1>Analytics page</h1> }));
vi.mock('../../pages/ApiExplorerPage', () => ({ ApiExplorerPage: () => <h1>API Explorer page</h1> }));
vi.mock('../../i18n/I18nProvider', () => ({ useI18n: () => ({ t: (key: string) => key === 'route.localOnly.title' ? 'Requires local MockForge' : key === 'nav.runLocally' ? 'Run locally' : key }) }));

import { routes } from '../index';
import { localOnlyRouteExceptions, localOnlyRouteIds } from '../../components/layout/navigation';

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

  it('guards every local-only route except explicit cloud exceptions', async () => {
    cloudMode.enabled = true;
    const routeById: Record<string, string> = {
      playground: '/playground', 'proxy-inspector': '/proxy-inspector', 'smtp-mailbox': '/smtp-mailbox',
      'mqtt-broker': '/mqtt-broker', 'kafka-broker': '/kafka-broker', 'amqp-broker': '/amqp-broker',
      chaos: '/chaos', recorder: '/recorder', 'behavioral-cloning': '/behavioral-cloning', incidents: '/incidents',
      traces: '/traces', metrics: '/metrics', performance: '/performance', 'contract-diff': '/contract-diff',
      plugins: '/plugins', 'test-execution': '/test-execution',
    };
    for (const id of localOnlyRouteIds) {
      if (localOnlyRouteExceptions.has(id)) continue;
      expect(routeById[id], `route for ${id}`).toBeDefined();
      const view = renderPath(routeById[id]);
      if (id === 'incidents') {
        expect(await screen.findByRole('heading', { name: 'Cloud incidents page' })).toBeVisible();
        expect(screen.getByTestId('path')).toHaveTextContent('/cloud-incidents');
      } else {
        expect(await screen.findByRole('heading', { name: 'Requires local MockForge' }), id).toBeVisible();
      }
      view.unmount();
    }
  });

  it('renders explicit local-only route exceptions in cloud mode', async () => {
    cloudMode.enabled = true;
    const analytics = renderPath('/analytics');
    expect(await screen.findByRole('heading', { name: 'Analytics page' })).toBeVisible();
    analytics.unmount();
    window.__mockforge_explorer_deployment = { id: 'test' } as never;
    renderPath('/api-explorer');
    expect(await screen.findByRole('heading', { name: 'API Explorer page' })).toBeVisible();
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
    renderPath('/incident-dashboard');
    expect(await screen.findByRole('heading', { name: 'Cloud incidents page' })).toBeVisible();
    expect(screen.getByTestId('path')).toHaveTextContent('/cloud-incidents');
  });

  it('renders the working playground page in local mode', async () => {
    cloudMode.enabled = false;
    renderPath('/playground');
    expect(await screen.findByRole('heading', { name: 'Playground page' })).toBeVisible();
  });
});
