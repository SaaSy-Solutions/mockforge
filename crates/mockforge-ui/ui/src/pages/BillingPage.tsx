import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  CreditCard,
  CheckCircle2,
  XCircle,
  Calendar,
  TrendingUp,
  HardDrive,
  Zap,
  AlertCircle,
  ExternalLink,
  ArrowUpCircle,
} from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';

// Types
interface Subscription {
  org_id: string;
  plan: 'free' | 'pro' | 'team';
  status: 'active' | 'trialing' | 'past_due' | 'canceled' | 'unpaid';
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
  };
}

interface UsageStats {
  requests: number;
  requests_limit: number;
  storage_bytes: number;
  storage_limit_bytes: number;
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

// API base URL - adjust based on your setup
const API_BASE = '/api/v1';

async function fetchSubscription(): Promise<Subscription> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/billing/subscription`, {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    throw new Error('Failed to fetch subscription');
  }
  return response.json();
}

async function createCheckout(request: CreateCheckoutRequest): Promise<CreateCheckoutResponse> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/billing/checkout`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(request),
  });
  if (!response.ok) {
    throw new Error('Failed to create checkout session');
  }
  return response.json();
}

export function BillingPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [selectedPlan, setSelectedPlan] = useState<'pro' | 'team' | null>(null);

  // Fetch subscription
  const { data: subscription, isLoading } = useQuery({
    queryKey: ['subscription'],
    queryFn: fetchSubscription,
  });

  // Create checkout mutation
  const checkoutMutation = useMutation({
    mutationFn: createCheckout,
    onSuccess: (data) => {
      // Redirect to Stripe Checkout
      window.location.href = data.checkout_url;
    },
    onError: (error: Error) => {
      showToast({
        title: 'Error',
        description: error.message || 'Failed to create checkout session',
        variant: 'destructive',
      });
    },
  });

  const handleUpgrade = (plan: 'pro' | 'team') => {
    setSelectedPlan(plan);
    checkoutMutation.mutate({
      plan,
      success_url: `${window.location.origin}/billing?success=true`,
      cancel_url: `${window.location.origin}/billing?canceled=true`,
    });
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  };

  const formatNumber = (num: number) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

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
    return (
      <div className="container mx-auto p-6">
        <div className="text-center py-12">Failed to load subscription</div>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Billing & Subscription</h1>
        <p className="text-muted-foreground mt-2">Manage your subscription and view usage</p>
      </div>

      <Tabs defaultValue="overview" className="space-y-4">
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="usage">Usage</TabsTrigger>
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
                  {subscription.current_period_end && (
                    <div className="text-sm text-muted-foreground mt-1">
                      Renews on {new Date(subscription.current_period_end).toLocaleDateString()}
                    </div>
                  )}
                </div>
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span>Projects</span>
                    <span>
                      {subscription.limits.max_projects === -1
                        ? 'Unlimited'
                        : subscription.limits.max_projects}
                    </span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span>Collaborators</span>
                    <span>{subscription.limits.max_collaborators}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span>Environments</span>
                    <span>{subscription.limits.max_environments}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span>Hosted Mocks</span>
                    <span>{subscription.limits.hosted_mocks ? 'Yes' : 'No'}</span>
                  </div>
                </div>
                {subscription.plan === 'free' && (
                  <Button
                    onClick={() => handleUpgrade('pro')}
                    className="w-full"
                    disabled={checkoutMutation.isPending}
                  >
                    <ArrowUpCircle className="w-4 h-4 mr-2" />
                    Upgrade to Pro
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
              <CardTitle>Detailed Usage</CardTitle>
              <CardDescription>View detailed usage statistics</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-6">
                {/* Requests */}
                <div>
                  <div className="flex justify-between items-center mb-2">
                    <h3 className="font-semibold flex items-center">
                      <TrendingUp className="w-4 h-4 mr-2" />
                      API Requests
                    </h3>
                    <span className="text-sm text-muted-foreground">
                      {formatNumber(subscription.usage.requests)} /{' '}
                      {formatNumber(subscription.usage.requests_limit)}
                    </span>
                  </div>
                  <div className="w-full bg-secondary rounded-full h-3">
                    <div
                      className={`h-3 rounded-full transition-all ${
                        getUsagePercentage(
                          subscription.usage.requests,
                          subscription.usage.requests_limit
                        ) > 90
                          ? 'bg-red-500'
                          : getUsagePercentage(
                              subscription.usage.requests,
                              subscription.usage.requests_limit
                            ) > 75
                          ? 'bg-yellow-500'
                          : 'bg-green-500'
                      }`}
                      style={{
                        width: `${getUsagePercentage(
                          subscription.usage.requests,
                          subscription.usage.requests_limit
                        )}%`,
                      }}
                    />
                  </div>
                </div>

                {/* Storage */}
                <div>
                  <div className="flex justify-between items-center mb-2">
                    <h3 className="font-semibold flex items-center">
                      <HardDrive className="w-4 h-4 mr-2" />
                      Storage
                    </h3>
                    <span className="text-sm text-muted-foreground">
                      {formatBytes(subscription.usage.storage_bytes)} /{' '}
                      {formatBytes(subscription.usage.storage_limit_bytes)}
                    </span>
                  </div>
                  <div className="w-full bg-secondary rounded-full h-3">
                    <div
                      className={`h-3 rounded-full transition-all ${
                        getUsagePercentage(
                          subscription.usage.storage_bytes,
                          subscription.usage.storage_limit_bytes
                        ) > 90
                          ? 'bg-red-500'
                          : getUsagePercentage(
                              subscription.usage.storage_bytes,
                              subscription.usage.storage_limit_bytes
                            ) > 75
                          ? 'bg-yellow-500'
                          : 'bg-green-500'
                      }`}
                      style={{
                        width: `${getUsagePercentage(
                          subscription.usage.storage_bytes,
                          subscription.usage.storage_limit_bytes
                        )}%`,
                      }}
                    />
                  </div>
                </div>

                {/* AI Tokens */}
                {subscription.usage.ai_tokens_limit > 0 && (
                  <div>
                    <div className="flex justify-between items-center mb-2">
                      <h3 className="font-semibold flex items-center">
                        <Zap className="w-4 h-4 mr-2" />
                        AI Tokens
                      </h3>
                      <span className="text-sm text-muted-foreground">
                        {formatNumber(subscription.usage.ai_tokens_used)} /{' '}
                        {formatNumber(subscription.usage.ai_tokens_limit)}
                      </span>
                    </div>
                    <div className="w-full bg-secondary rounded-full h-3">
                      <div
                        className={`h-3 rounded-full transition-all ${
                          getUsagePercentage(
                            subscription.usage.ai_tokens_used,
                            subscription.usage.ai_tokens_limit
                          ) > 90
                            ? 'bg-red-500'
                            : getUsagePercentage(
                                subscription.usage.ai_tokens_used,
                                subscription.usage.ai_tokens_limit
                              ) > 75
                            ? 'bg-yellow-500'
                            : 'bg-green-500'
                        }`}
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
              </div>
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
                  <Button variant="outline" className="w-full" disabled>
                    Downgrade
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
