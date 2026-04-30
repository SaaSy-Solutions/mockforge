import React, { useState, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import {
  CheckCircle2,
  XCircle,
  Calendar,
  TrendingUp,
  HardDrive,
  Zap,
  Activity,
  AlertCircle,
  ExternalLink,
  ArrowUpCircle,
  CreditCard,
} from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';
import { authenticatedFetch } from '@/utils/apiClient';

// Types
interface Subscription {
  org_id: string;
  plan: 'free' | 'pro' | 'team';
  status:
    | 'active'
    | 'trialing'
    | 'past_due'
    | 'canceled'
    | 'unpaid'
    | 'incomplete'
    | 'incomplete_expired';
  cancel_at_period_end?: boolean;
  current_period_start?: string;
  current_period_end?: string;
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

interface UsageStats {
  requests: number;
  requests_limit: number;
  storage_bytes: number;
  storage_limit_bytes: number;
  egress_bytes: number;
  egress_limit_bytes: number;
  ai_tokens_used: number;
  ai_tokens_limit: number;
}

interface CreateCheckoutRequest {
  plan: 'pro' | 'team';
  success_url?: string;
  cancel_url?: string;
}

interface CreateCheckoutResponse {
  checkout_url: string;
}

interface CreatePortalResponse {
  portal_url: string;
}

interface InvoiceItem {
  id: string;
  number: string | null;
  status: string | null;
  amount_due: number;
  amount_paid: number;
  currency: string | null;
  created: number | null;
  period_start: number | null;
  period_end: number | null;
  hosted_invoice_url: string | null;
  invoice_pdf: string | null;
}

interface ListInvoicesResponse {
  org_id: string;
  invoices: InvoiceItem[];
}

// API base URL - adjust based on your setup
const API_BASE = '/api/v1';

async function extractErrorMessage(response: Response, fallback: string): Promise<string> {
  try {
    const body = await response.json();
    if (body && typeof body === 'object') {
      const msg = body.error ?? body.message ?? body.details?.message;
      if (typeof msg === 'string' && msg.trim().length > 0) {
        return msg;
      }
    }
  } catch {
    // Non-JSON body — fall through.
  }
  return fallback;
}

async function fetchSubscription(): Promise<Subscription> {
  const response = await authenticatedFetch(`${API_BASE}/billing/subscription`, {
    headers: { 'Content-Type': 'application/json' },
  });
  if (!response.ok) {
    throw new Error(await extractErrorMessage(response, 'Failed to fetch subscription'));
  }
  return response.json();
}

async function createCheckout(request: CreateCheckoutRequest): Promise<CreateCheckoutResponse> {
  const response = await authenticatedFetch(`${API_BASE}/billing/checkout`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  });
  if (!response.ok) {
    throw new Error(await extractErrorMessage(response, 'Failed to create checkout session'));
  }
  return response.json();
}

async function fetchInvoices(): Promise<ListInvoicesResponse> {
  const response = await authenticatedFetch(`${API_BASE}/billing/invoices`, {
    headers: { 'Content-Type': 'application/json' },
  });
  if (!response.ok) {
    throw new Error(await extractErrorMessage(response, 'Failed to fetch invoices'));
  }
  return response.json();
}

async function createPortalSession(): Promise<CreatePortalResponse> {
  const response = await authenticatedFetch(`${API_BASE}/billing/portal`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      return_url: window.location.href,
    }),
  });
  if (!response.ok) {
    throw new Error(await extractErrorMessage(response, 'Failed to create portal session'));
  }
  return response.json();
}

export function BillingPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [selectedPlan, setSelectedPlan] = useState<'pro' | 'team' | null>(null);

  // Fetch subscription
  const {
    data: subscription,
    isLoading,
    error: subscriptionError,
    refetch: refetchSubscription,
  } = useQuery({
    queryKey: ['subscription'],
    queryFn: fetchSubscription,
  });

  // Fetch invoices (only meaningful once we have a Stripe customer; backend
  // returns an empty list otherwise so this is safe to fire eagerly)
  const {
    data: invoiceList,
    isLoading: invoicesLoading,
    isError: invoicesError,
    refetch: refetchInvoices,
  } = useQuery({
    queryKey: ['invoices'],
    queryFn: fetchInvoices,
    staleTime: 5 * 60_000,
  });

  // Create checkout mutation
  const checkoutMutation = useMutation({
    mutationFn: createCheckout,
    onSuccess: (data) => {
      // Redirect to Stripe Checkout
      window.location.href = data.checkout_url;
    },
    onError: (error: Error) => {
      showToast('error', 'Error', error.message || 'Failed to create checkout session');
    },
  });

  // Create portal session mutation
  const portalMutation = useMutation({
    mutationFn: createPortalSession,
    onSuccess: (data) => {
      window.location.href = data.portal_url;
    },
    onError: (error: Error) => {
      showToast('error', 'Error', error.message || 'Failed to open billing portal');
    },
  });

  const handleManageSubscription = () => {
    portalMutation.mutate();
  };

  const handleUpgrade = (plan: 'pro' | 'team') => {
    setSelectedPlan(plan);
    checkoutMutation.mutate({
      plan,
      success_url: `${window.location.origin}/billing?success=true`,
      cancel_url: `${window.location.origin}/billing?canceled=true`,
    });
  };

  // Handle Stripe checkout redirect query params
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    if (params.get('success') === 'true') {
      showToast('success', 'Subscription Updated', 'Your subscription has been updated successfully.');
      queryClient.invalidateQueries({ queryKey: ['subscription'] });
      window.history.replaceState({}, '', '/billing');
    } else if (params.get('canceled') === 'true') {
      showToast('info', 'Checkout Canceled', 'Your checkout session was canceled.');
      window.history.replaceState({}, '', '/billing');
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1000;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  const formatNumber = (num: number) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  // -1 = unlimited, 0 = not included, otherwise show the number
  const formatLimit = (n: number) => (n === -1 ? 'Unlimited' : n === 0 ? 'Not included' : String(n));

  const getUsagePercentage = (used: number, limit: number) => {
    if (limit <= 0) return 0;
    return Math.min((used / limit) * 100, 100);
  };

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'active':
        return <Badge className="bg-green-500"><CheckCircle2 className="w-3 h-3 mr-1" />Active</Badge>;
      case 'trialing':
        return <Badge className="bg-blue-500"><Calendar className="w-3 h-3 mr-1" />Trialing</Badge>;
      case 'past_due':
        return <Badge className="bg-yellow-500"><AlertCircle className="w-3 h-3 mr-1" />Past Due</Badge>;
      case 'unpaid':
        return <Badge className="bg-red-500"><AlertCircle className="w-3 h-3 mr-1" />Unpaid</Badge>;
      case 'incomplete':
        return <Badge className="bg-yellow-500"><AlertCircle className="w-3 h-3 mr-1" />Incomplete</Badge>;
      case 'incomplete_expired':
        return <Badge className="bg-gray-500"><XCircle className="w-3 h-3 mr-1" />Incomplete (expired)</Badge>;
      case 'canceled':
        return <Badge className="bg-gray-500"><XCircle className="w-3 h-3 mr-1" />Canceled</Badge>;
      default:
        return <Badge>{status}</Badge>;
    }
  };

  if (isLoading) {
    return (
      <div className="container mx-auto p-6">
        <div className="text-center py-12">Loading subscription...</div>
      </div>
    );
  }

  if (!subscription) {
    const message =
      subscriptionError instanceof Error
        ? subscriptionError.message
        : 'Failed to load subscription';
    return (
      <div className="container mx-auto p-6">
        <div className="mx-auto max-w-md text-center py-12 space-y-4">
          <AlertCircle className="w-8 h-8 text-yellow-600 mx-auto" />
          <div>
            <h2 className="font-semibold">Failed to load subscription</h2>
            <p className="text-sm text-muted-foreground mt-1">{message}</p>
          </div>
          <Button variant="outline" onClick={() => refetchSubscription()}>
            Retry
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Billing & Subscription</h1>
        <p className="text-muted-foreground mt-2">Manage your subscription and view usage</p>
      </div>

      {subscription.cancel_at_period_end && subscription.current_period_end && (
        <Card className="border-yellow-500 bg-yellow-50 dark:bg-yellow-950/20">
          <CardContent className="p-4">
            <div className="flex items-start space-x-3">
              <AlertCircle className="w-5 h-5 text-yellow-600 mt-0.5" />
              <div>
                <h3 className="font-semibold text-yellow-800 dark:text-yellow-200">Subscription Canceling</h3>
                <p className="text-sm text-yellow-700 dark:text-yellow-300">
                  Your subscription will be canceled at the end of the current billing period on{' '}
                  {new Date(subscription.current_period_end).toLocaleDateString()}. You will retain
                  access to your current plan until then.
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      <PaymentIssueBanner
        status={subscription.status}
        onUpdatePaymentMethod={handleManageSubscription}
        portalPending={portalMutation.isPending}
      />

      <Tabs defaultValue="overview" className="space-y-4">
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="usage">Usage</TabsTrigger>
          <TabsTrigger value="invoices">Invoices</TabsTrigger>
          <TabsTrigger value="plans">Plans</TabsTrigger>
        </TabsList>

        {/* Overview Tab */}
        <TabsContent value="overview" className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            {/* Current Plan */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center justify-between">
                  <span>Current Plan</span>
                  {getStatusBadge(subscription.status)}
                </CardTitle>
                <CardDescription>Your active subscription details</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <div className="text-2xl font-bold capitalize">{subscription.plan}</div>
                  {subscription.current_period_start && subscription.current_period_end && (
                    <div className="text-sm text-muted-foreground mt-1">
                      Current period:{' '}
                      {new Date(subscription.current_period_start).toLocaleDateString()} –{' '}
                      {new Date(subscription.current_period_end).toLocaleDateString()}
                    </div>
                  )}
                  {subscription.current_period_end && (
                    <div className="text-sm text-muted-foreground mt-1">
                      {subscription.cancel_at_period_end ? 'Ends on ' : 'Renews on '}
                      {new Date(subscription.current_period_end).toLocaleDateString()}
                    </div>
                  )}
                </div>
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span>Projects</span>
                    <span>{formatLimit(subscription.limits.max_projects)}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span>Collaborators</span>
                    <span>{formatLimit(subscription.limits.max_collaborators)}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span>Environments</span>
                    <span>{formatLimit(subscription.limits.max_environments)}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span>Hosted Mocks</span>
                    <span>
                      {subscription.limits.hosted_mocks
                        ? formatLimit(subscription.limits.max_hosted_mocks)
                        : 'Not included'}
                    </span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span>Plugins published</span>
                    <span>{formatLimit(subscription.limits.max_plugins_published)}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span>Templates published</span>
                    <span>{formatLimit(subscription.limits.max_templates_published)}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span>Scenarios published</span>
                    <span>{formatLimit(subscription.limits.max_scenarios_published)}</span>
                  </div>
                </div>
                {subscription.plan === 'free' ? (
                  <Button
                    onClick={() => handleUpgrade('pro')}
                    className="w-full"
                    disabled={checkoutMutation.isPending}
                  >
                    <ArrowUpCircle className="w-4 h-4 mr-2" />
                    Upgrade to Pro
                  </Button>
                ) : (
                  <Button
                    variant="outline"
                    onClick={handleManageSubscription}
                    className="w-full"
                    disabled={portalMutation.isPending}
                  >
                    <ExternalLink className="w-4 h-4 mr-2" />
                    {portalMutation.isPending ? 'Opening...' : 'Manage Subscription'}
                  </Button>
                )}
              </CardContent>
            </Card>

            {/* Quick Usage Stats */}
            <Card>
              <CardHeader>
                <CardTitle>Usage This Month</CardTitle>
                <CardDescription>Current usage against your plan limits</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <div className="flex justify-between text-sm mb-1">
                    <span className="flex items-center">
                      <TrendingUp className="w-4 h-4 mr-1" />
                      Requests
                    </span>
                    <span>
                      {formatNumber(subscription.usage.requests)} /{' '}
                      {formatNumber(subscription.usage.requests_limit)}
                    </span>
                  </div>
                  <div className="w-full bg-secondary rounded-full h-2">
                    <div
                      className="bg-primary h-2 rounded-full transition-all"
                      style={{
                        width: `${getUsagePercentage(
                          subscription.usage.requests,
                          subscription.usage.requests_limit
                        )}%`,
                      }}
                    />
                  </div>
                </div>
                <div>
                  <div className="flex justify-between text-sm mb-1">
                    <span className="flex items-center">
                      <HardDrive className="w-4 h-4 mr-1" />
                      Storage
                    </span>
                    <span>
                      {formatBytes(subscription.usage.storage_bytes)} /{' '}
                      {formatBytes(subscription.usage.storage_limit_bytes)}
                    </span>
                  </div>
                  <div className="w-full bg-secondary rounded-full h-2">
                    <div
                      className="bg-primary h-2 rounded-full transition-all"
                      style={{
                        width: `${getUsagePercentage(
                          subscription.usage.storage_bytes,
                          subscription.usage.storage_limit_bytes
                        )}%`,
                      }}
                    />
                  </div>
                </div>
                {(subscription.usage.egress_limit_bytes > 0 || subscription.usage.egress_bytes > 0) && (
                  <div>
                    <div className="flex justify-between text-sm mb-1">
                      <span className="flex items-center">
                        <Activity className="w-4 h-4 mr-1" />
                        Egress
                      </span>
                      <span>
                        {formatBytes(subscription.usage.egress_bytes)}
                        {subscription.usage.egress_limit_bytes > 0
                          ? ` / ${formatBytes(subscription.usage.egress_limit_bytes)}`
                          : ''}
                      </span>
                    </div>
                    {subscription.usage.egress_limit_bytes > 0 && (
                      <div className="w-full bg-secondary rounded-full h-2">
                        <div
                          className="bg-primary h-2 rounded-full transition-all"
                          style={{
                            width: `${getUsagePercentage(
                              subscription.usage.egress_bytes,
                              subscription.usage.egress_limit_bytes
                            )}%`,
                          }}
                        />
                      </div>
                    )}
                  </div>
                )}
                {subscription.usage.ai_tokens_limit > 0 && (
                  <div>
                    <div className="flex justify-between text-sm mb-1">
                      <span className="flex items-center">
                        <Zap className="w-4 h-4 mr-1" />
                        AI Tokens
                      </span>
                      <span>
                        {formatNumber(subscription.usage.ai_tokens_used)} /{' '}
                        {formatNumber(subscription.usage.ai_tokens_limit)}
                      </span>
                    </div>
                    <div className="w-full bg-secondary rounded-full h-2">
                      <div
                        className="bg-primary h-2 rounded-full transition-all"
                        style={{
                          width: `${getUsagePercentage(
                            subscription.usage.ai_tokens_used,
                            subscription.usage.ai_tokens_limit
                          )}%`,
                        }}
                      />
                    </div>
                  </div>
                )}
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        {/* Usage Tab */}
        <TabsContent value="usage" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center">
                <TrendingUp className="w-5 h-5 mr-2" />
                Usage Dashboard
              </CardTitle>
              <CardDescription>
                View detailed usage statistics, history, and per-metric breakdowns
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-3 sm:grid-cols-3">
                <div className="rounded-lg border p-3">
                  <div className="text-sm text-muted-foreground">Requests</div>
                  <div className="text-lg font-semibold">
                    {formatNumber(subscription.usage.requests)} / {formatNumber(subscription.usage.requests_limit)}
                  </div>
                </div>
                <div className="rounded-lg border p-3">
                  <div className="text-sm text-muted-foreground">Storage</div>
                  <div className="text-lg font-semibold">
                    {formatBytes(subscription.usage.storage_bytes)} / {formatBytes(subscription.usage.storage_limit_bytes)}
                  </div>
                </div>
                {(subscription.usage.egress_limit_bytes > 0 || subscription.usage.egress_bytes > 0) && (
                  <div className="rounded-lg border p-3">
                    <div className="text-sm text-muted-foreground">Egress</div>
                    <div className="text-lg font-semibold">
                      {formatBytes(subscription.usage.egress_bytes)}
                      {subscription.usage.egress_limit_bytes > 0
                        ? ` / ${formatBytes(subscription.usage.egress_limit_bytes)}`
                        : ''}
                    </div>
                  </div>
                )}
                {subscription.usage.ai_tokens_limit > 0 && (
                  <div className="rounded-lg border p-3">
                    <div className="text-sm text-muted-foreground">AI Tokens</div>
                    <div className="text-lg font-semibold">
                      {formatNumber(subscription.usage.ai_tokens_used)} / {formatNumber(subscription.usage.ai_tokens_limit)}
                    </div>
                  </div>
                )}
              </div>
              <a
                href="/usage"
                className="inline-flex items-center text-sm font-medium text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
              >
                View full usage dashboard
                <ExternalLink className="w-3 h-3 ml-1" />
              </a>
            </CardContent>
          </Card>
        </TabsContent>

        {/* Invoices Tab */}
        <TabsContent value="invoices" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Invoices</CardTitle>
              <CardDescription>
                Past invoices from Stripe. Use the Stripe portal for payment-method changes.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {invoicesLoading ? (
                <div className="text-center py-6 text-sm text-muted-foreground">
                  Loading invoices...
                </div>
              ) : invoicesError ? (
                <div className="text-center py-6 space-y-3">
                  <AlertCircle className="w-8 h-8 mx-auto text-red-500" />
                  <p className="text-sm text-muted-foreground">Failed to load invoices</p>
                  <Button variant="outline" size="sm" onClick={() => refetchInvoices()}>
                    Retry
                  </Button>
                </div>
              ) : !invoiceList || invoiceList.invoices.length === 0 ? (
                <div className="text-center py-6 text-sm text-muted-foreground">
                  No invoices yet.{' '}
                  {subscription.plan === 'free'
                    ? 'Upgrade to a paid plan to start receiving invoices.'
                    : 'Your first invoice will appear after the next billing cycle.'}
                </div>
              ) : (
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b text-left text-muted-foreground">
                        <th className="py-2 pr-4 font-medium">Date</th>
                        <th className="py-2 pr-4 font-medium">Number</th>
                        <th className="py-2 pr-4 font-medium">Period</th>
                        <th className="py-2 pr-4 font-medium">Amount</th>
                        <th className="py-2 pr-4 font-medium">Status</th>
                        <th className="py-2 pr-4 font-medium text-right">Links</th>
                      </tr>
                    </thead>
                    <tbody>
                      {invoiceList.invoices.map((inv) => (
                        <tr key={inv.id} className="border-b last:border-0">
                          <td className="py-2 pr-4">{formatUnix(inv.created)}</td>
                          <td className="py-2 pr-4 font-mono text-xs">{inv.number ?? '—'}</td>
                          <td className="py-2 pr-4 text-xs text-muted-foreground">
                            {formatPeriod(inv.period_start, inv.period_end)}
                          </td>
                          <td className="py-2 pr-4 font-medium">
                            {formatMoney(inv.amount_paid || inv.amount_due, inv.currency)}
                          </td>
                          <td className="py-2 pr-4">
                            <InvoiceStatusBadge status={inv.status} />
                          </td>
                          <td className="py-2 pr-4 text-right space-x-2">
                            {inv.hosted_invoice_url && (
                              <a
                                href={inv.hosted_invoice_url}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-blue-600 hover:underline dark:text-blue-400"
                              >
                                View
                              </a>
                            )}
                            {inv.invoice_pdf && (
                              <a
                                href={inv.invoice_pdf}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-blue-600 hover:underline dark:text-blue-400"
                              >
                                PDF
                              </a>
                            )}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Plans Tab */}
        <TabsContent value="plans" className="space-y-4">
          <div className="grid gap-4 md:grid-cols-3">
            {/* Free Plan */}
            <Card className={subscription.plan === 'free' ? 'border-primary' : ''}>
              <CardHeader>
                <CardTitle>Free</CardTitle>
                <div className="text-3xl font-bold">$0</div>
                <CardDescription>per month</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <ul className="space-y-2 text-sm">
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    1 Project
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    1 Collaborator
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    10K requests/month
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    1GB storage
                  </li>
                  <li className="flex items-center">
                    <XCircle className="w-4 h-4 mr-2 text-gray-400" />
                    BYOK only for AI
                  </li>
                </ul>
                {subscription.plan === 'free' ? (
                  <Button disabled className="w-full">Current Plan</Button>
                ) : (
                  <Button
                    variant="outline"
                    className="w-full"
                    onClick={handleManageSubscription}
                    disabled={portalMutation.isPending}
                  >
                    <ExternalLink className="w-4 h-4 mr-2" />
                    {portalMutation.isPending ? 'Opening...' : 'Manage via Stripe'}
                  </Button>
                )}
              </CardContent>
            </Card>

            {/* Pro Plan */}
            <Card className={subscription.plan === 'pro' ? 'border-primary' : ''}>
              <CardHeader>
                <CardTitle>Pro</CardTitle>
                <div className="text-3xl font-bold">$19</div>
                <CardDescription>per month</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <ul className="space-y-2 text-sm">
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    10 Projects
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    5 Collaborators
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    250K requests/month
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    20GB storage
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    100K AI tokens/month
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    Hosted mocks
                  </li>
                </ul>
                {subscription.plan === 'pro' ? (
                  <Button disabled className="w-full">Current Plan</Button>
                ) : (
                  <Button
                    onClick={() => handleUpgrade('pro')}
                    className="w-full"
                    disabled={checkoutMutation.isPending}
                  >
                    {checkoutMutation.isPending && selectedPlan === 'pro'
                      ? 'Processing...'
                      : 'Upgrade to Pro'}
                  </Button>
                )}
              </CardContent>
            </Card>

            {/* Team Plan */}
            <Card className={subscription.plan === 'team' ? 'border-primary' : ''}>
              <CardHeader>
                <CardTitle>Team</CardTitle>
                <div className="text-3xl font-bold">$79</div>
                <CardDescription>per month</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <ul className="space-y-2 text-sm">
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    Unlimited Projects
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    20 Collaborators
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    1M requests/month
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    100GB storage
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    1M AI tokens/month
                  </li>
                  <li className="flex items-center">
                    <CheckCircle2 className="w-4 h-4 mr-2 text-green-500" />
                    Hosted mocks
                  </li>
                </ul>
                {subscription.plan === 'team' ? (
                  <Button disabled className="w-full">Current Plan</Button>
                ) : (
                  <Button
                    onClick={() => handleUpgrade('team')}
                    className="w-full"
                    disabled={checkoutMutation.isPending}
                  >
                    {checkoutMutation.isPending && selectedPlan === 'team'
                      ? 'Processing...'
                      : 'Upgrade to Team'}
                  </Button>
                )}
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

function formatUnix(unixSeconds: number | null): string {
  if (!unixSeconds) return '—';
  return new Date(unixSeconds * 1000).toLocaleDateString();
}

function formatPeriod(start: number | null, end: number | null): string {
  if (!start || !end) return '—';
  const s = new Date(start * 1000).toLocaleDateString();
  const e = new Date(end * 1000).toLocaleDateString();
  return `${s} – ${e}`;
}

function formatMoney(cents: number, currency: string | null): string {
  const code = (currency ?? 'usd').toUpperCase();
  const amount = cents / 100;
  try {
    return new Intl.NumberFormat(undefined, { style: 'currency', currency: code }).format(amount);
  } catch {
    return `${amount.toFixed(2)} ${code}`;
  }
}

interface PaymentIssueBannerProps {
  status: Subscription['status'];
  onUpdatePaymentMethod: () => void;
  portalPending: boolean;
}

// Shown when Stripe reports the subscription is in a non-active state that
// requires user action (failed payment, incomplete authentication, etc.).
// Backend webhooks set these statuses; the banner offers a one-click jump
// to the Stripe portal to update the payment method.
function PaymentIssueBanner({ status, onUpdatePaymentMethod, portalPending }: PaymentIssueBannerProps) {
  let title: string | null = null;
  let message: string | null = null;
  let cta = 'Update Payment Method';
  let tone: 'red' | 'yellow' = 'yellow';

  switch (status) {
    case 'past_due':
      tone = 'red';
      title = 'Payment Past Due';
      message =
        'Your most recent payment failed. Update your payment method to keep your subscription active — Stripe will retry the charge automatically.';
      break;
    case 'unpaid':
      tone = 'red';
      title = 'Subscription Unpaid';
      message =
        'Your subscription is unpaid and access will be reduced soon. Update your payment method to restore service.';
      break;
    case 'incomplete':
      tone = 'yellow';
      title = 'Payment Incomplete';
      message =
        'Your last payment requires additional action (such as 3D Secure authentication). Open the billing portal to complete it.';
      cta = 'Complete Payment';
      break;
    case 'incomplete_expired':
      tone = 'red';
      title = 'Subscription Setup Failed';
      message =
        'The initial payment for this subscription expired before it could be confirmed. Start a new checkout to subscribe.';
      cta = 'Manage in Stripe';
      break;
    default:
      return null;
  }

  const toneClasses =
    tone === 'red'
      ? 'border-red-500 bg-red-50 dark:bg-red-950/20'
      : 'border-yellow-500 bg-yellow-50 dark:bg-yellow-950/20';
  const iconClass = tone === 'red' ? 'text-red-600' : 'text-yellow-600';
  const headingClass =
    tone === 'red'
      ? 'text-red-800 dark:text-red-200'
      : 'text-yellow-800 dark:text-yellow-200';
  const bodyClass =
    tone === 'red' ? 'text-red-700 dark:text-red-300' : 'text-yellow-700 dark:text-yellow-300';

  return (
    <Card className={toneClasses}>
      <CardContent className="p-4">
        <div className="flex items-start gap-3">
          <AlertCircle className={`w-5 h-5 mt-0.5 ${iconClass}`} />
          <div className="flex-1">
            <h3 className={`font-semibold ${headingClass}`}>{title}</h3>
            <p className={`text-sm ${bodyClass}`}>{message}</p>
          </div>
          <Button
            variant="outline"
            onClick={onUpdatePaymentMethod}
            disabled={portalPending}
          >
            <CreditCard className="w-4 h-4 mr-2" />
            {portalPending ? 'Opening...' : cta}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function InvoiceStatusBadge({ status }: { status: string | null }) {
  if (!status) return <Badge>—</Badge>;
  switch (status) {
    case 'paid':
      return <Badge className="bg-green-500"><CheckCircle2 className="w-3 h-3 mr-1" />Paid</Badge>;
    case 'open':
      return <Badge className="bg-yellow-500"><AlertCircle className="w-3 h-3 mr-1" />Open</Badge>;
    case 'uncollectible':
      return <Badge className="bg-red-500"><XCircle className="w-3 h-3 mr-1" />Uncollectible</Badge>;
    case 'void':
      return <Badge className="bg-gray-500"><XCircle className="w-3 h-3 mr-1" />Void</Badge>;
    case 'draft':
      return <Badge className="bg-gray-400">Draft</Badge>;
    default:
      return <Badge>{status}</Badge>;
  }
}
