/** Types and normalization for GET /api/v1/billing/subscription. */

export interface Subscription {
  org_id: string;
  plan: 'free' | 'pro' | 'team';
  /**
   * Stripe subscription status, or `'none'` when the org's plan is not backed
   * by a Stripe subscription (Free, or a paid plan assigned administratively).
   */
  status:
    | 'none'
    | 'active'
    | 'trialing'
    | 'past_due'
    | 'canceled'
    | 'unpaid'
    | 'incomplete'
    | 'incomplete_expired';
  billing_interval?: 'month' | 'year' | null;
  cancel_at_period_end?: boolean;
  current_period_start?: string | null;
  current_period_end?: string | null;
  usage: UsageStats;
  limits: {
    max_projects: number;
    max_collaborators: number;
    max_environments: number;
    requests_per_30d: number;
    storage_gb: number;
    ai_tokens_per_month: number;
    hosted_mocks: boolean;
    max_hosted_mocks: number;
    max_plugins_published: number;
    max_templates_published: number;
    max_scenarios_published: number;
  };
}

export interface UsageStats {
  requests: number;
  requests_limit: number;
  storage_bytes: number;
  storage_limit_bytes: number;
  egress_bytes: number;
  egress_limit_bytes: number;
  ai_tokens_used: number;
  ai_tokens_limit: number;
}

export type RawSubscription = Omit<Subscription, 'status'> & { status: Subscription['status'] | 'free' };

/**
 * Make "no Stripe subscription" explicit. `plan` is the authoritative plan
 * (it is what the limits are derived from); the Stripe fields only describe
 * a billing subscription when one exists. Older registry builds report a
 * missing subscription as `status: "free"` (even for paid plans) with a
 * fabricated renewal date and a default monthly interval, which made the
 * card read "Current Plan [free] / Pro / Renews on ... / Billed monthly".
 */
export function normalizeSubscription(raw: RawSubscription): Subscription {
  const hasStripeSubscription =
    raw.status !== 'free' && raw.status !== 'none' && !!raw.current_period_start;
  if (hasStripeSubscription) {
    return raw as Subscription;
  }
  return {
    ...raw,
    status: 'none',
    billing_interval: null,
    cancel_at_period_end: false,
    current_period_start: null,
    current_period_end: null,
  };
}
