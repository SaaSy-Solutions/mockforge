import { render, screen, within } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { normalizeSubscription, type RawSubscription } from '@/utils/billingSubscription';

const responses = new Map<string, unknown>();

vi.mock('@/utils/apiClient', () => ({
  authenticatedFetch: vi.fn(async (url: string) => {
    const path = new URL(url, 'http://localhost').pathname;
    const body = responses.get(path) ?? {};
    return new Response(JSON.stringify(body), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    });
  }),
}));

vi.mock('@/components/ui/ToastProvider', () => ({ useToast: () => ({ showToast: vi.fn() }) }));

import { BillingPage } from '../BillingPage';

const limits = {
  max_projects: -1,
  max_collaborators: 10,
  max_environments: 10,
  requests_per_30d: 1_000_000,
  storage_gb: 50,
  ai_tokens_per_month: 1_000_000,
  hosted_mocks: true,
  max_hosted_mocks: 10,
  max_plugins_published: 10,
  max_templates_published: 10,
  max_scenarios_published: 10,
};

const usage = {
  requests: 0,
  requests_limit: 1_000_000,
  storage_bytes: 0,
  storage_limit_bytes: 50_000_000_000,
  egress_bytes: 0,
  egress_limit_bytes: -1,
  ai_tokens_used: 0,
  ai_tokens_limit: 1_000_000,
};

// What the currently deployed registry returns for a Pro org with no Stripe
// subscription row (e.g. plan assigned administratively).
const legacyProWithoutSubscription: RawSubscription = {
  org_id: 'org-1',
  plan: 'pro',
  status: 'free',
  billing_interval: 'month',
  cancel_at_period_end: false,
  current_period_start: null,
  current_period_end: '2027-09-27T00:00:00Z',
  usage,
  limits,
};

function renderBilling() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <BillingPage />
    </QueryClientProvider>,
  );
}

describe('normalizeSubscription', () => {
  it('treats a legacy "free" status as no Stripe subscription and drops fabricated fields', () => {
    const sub = normalizeSubscription(legacyProWithoutSubscription);
    expect(sub.plan).toBe('pro');
    expect(sub.status).toBe('none');
    expect(sub.current_period_end).toBeNull();
    expect(sub.billing_interval).toBeNull();
  });

  it('passes a real Stripe subscription through unchanged', () => {
    const raw: RawSubscription = {
      ...legacyProWithoutSubscription,
      status: 'active',
      current_period_start: '2026-09-01T00:00:00Z',
      current_period_end: '2026-10-01T00:00:00Z',
    };
    expect(normalizeSubscription(raw)).toEqual(raw);
  });
});

describe('BillingPage overview', () => {
  beforeEach(() => {
    responses.clear();
    responses.set('/api/v1/billing/invoices', { invoices: [] });
    responses.set('/api/v1/billing/config', { trial_period_days: 14, annual_billing_available: false });
  });

  it('does not contradict the plan when the org has no Stripe subscription', async () => {
    responses.set('/api/v1/billing/subscription', legacyProWithoutSubscription);
    renderBilling();

    const title = (await screen.findByText('Current Plan', { selector: 'span' })).parentElement!;
    expect(within(title).queryByText(/free/i)).not.toBeInTheDocument();
    expect(within(title).getByText('Active')).toBeInTheDocument();
    expect(screen.getAllByText('pro').length).toBeGreaterThan(0);
    expect(screen.queryByText(/Renews on/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Billed monthly/)).not.toBeInTheDocument();
    expect(screen.getByText('Not billed through a subscription')).toBeInTheDocument();
  });

  it('shows Stripe status and renewal for a real subscription', async () => {
    responses.set('/api/v1/billing/subscription', {
      ...legacyProWithoutSubscription,
      status: 'trialing',
      current_period_start: '2026-09-01T00:00:00Z',
      current_period_end: '2026-10-01T00:00:00Z',
    });
    renderBilling();

    const title = (await screen.findByText('Current Plan', { selector: 'span' })).parentElement!;
    expect(within(title).getByText('Trialing')).toBeInTheDocument();
    expect(screen.getByText(/Renews on/)).toBeInTheDocument();
    expect(screen.getByText(/Billed monthly/)).toBeInTheDocument();
  });
});
