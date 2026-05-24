import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { CheckCircle2, XCircle, ArrowRight } from 'lucide-react';

// Public billing-config endpoint. Returns trial length so the pricing
// CTA copy can stay in sync with the operator-set
// STRIPE_TRIAL_PERIOD_DAYS env var without redeploying the UI.
type BillingConfig = { trial_period_days: number };

async function fetchBillingConfig(): Promise<BillingConfig> {
  const res = await fetch('/api/v1/billing/config');
  if (!res.ok) {
    throw new Error(`billing config fetch failed: ${res.status}`);
  }
  return res.json();
}

export function PricingPage() {
  // Soft-fail: if the endpoint isn't reachable (offline, OSS build,
  // unconfigured), fall back to a sensible default rather than hiding
  // the price page. 14 matches the backend default in config.rs.
  const { data: billingConfig } = useQuery({
    queryKey: ['billing-config'],
    queryFn: fetchBillingConfig,
    staleTime: 5 * 60 * 1000,
    retry: 1,
  });
  const trialDays = billingConfig?.trial_period_days ?? 14;
  const hasTrial = trialDays > 0;
  const trialSubtitle = hasTrial ? `${trialDays}-day free trial · no commitment` : null;
  const paidCtaLabel = hasTrial ? 'Start free trial' : 'Upgrade';

  const handleGetStarted = () => {
    // Navigate to sign up or login
    window.location.href = '/signup';
  };

  const handleUpgrade = (plan: 'pro' | 'team') => {
    // Navigate to billing page (requires auth)
    // If not authenticated, redirect to login first
    window.location.href = '/login?redirect=/billing';
  };

  return (
    <div className="container mx-auto px-4 py-12 max-w-7xl">
      {/* Header */}
      <div className="text-center mb-12">
        <h1 className="text-4xl font-bold mb-4">Simple, Transparent Pricing</h1>
        <p className="text-xl text-muted-foreground max-w-2xl mx-auto">
          Choose the plan that's right for you. Start free, upgrade when you need more.
        </p>
      </div>

      {/* Pricing Cards */}
      <div className="grid gap-8 md:grid-cols-3 mb-12">
        {/* Free Plan */}
        <Card className="relative">
          <CardHeader>
            <CardTitle className="text-2xl">Free</CardTitle>
            <div className="mt-4">
              <span className="text-4xl font-bold">$0</span>
              <span className="text-muted-foreground">/month</span>
            </div>
            <CardDescription>Perfect for getting started</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <ul className="space-y-3">
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>10,000 API requests/month</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>1 GB storage</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>1 project</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>1 collaborator</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>BYOK for AI features</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>Basic marketplace access</span>
              </li>
              <li className="flex items-start">
                <XCircle className="w-5 h-5 mr-2 text-muted-foreground flex-shrink-0 mt-0.5" />
                <span className="text-muted-foreground">No hosted mocks</span>
              </li>
              <li className="flex items-start">
                <XCircle className="w-5 h-5 mr-2 text-muted-foreground flex-shrink-0 mt-0.5" />
                <span className="text-muted-foreground">Community support</span>
              </li>
            </ul>
            <Button className="w-full" onClick={handleGetStarted}>
              Get Started Free
            </Button>
          </CardContent>
        </Card>

        {/* Pro Plan */}
        <Card className="relative border-primary shadow-lg">
          <div className="absolute top-0 right-0 bg-primary text-primary-foreground px-3 py-1 rounded-bl-lg text-sm font-semibold">
            Most Popular
          </div>
          <CardHeader>
            <CardTitle className="text-2xl">Pro</CardTitle>
            <div className="mt-4">
              <span className="text-4xl font-bold">$29</span>
              <span className="text-muted-foreground">/month</span>
            </div>
            {trialSubtitle && (
              <p className="text-sm text-success-600 font-medium mt-1">{trialSubtitle}</p>
            )}
            <CardDescription>For professional developers</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <ul className="space-y-3">
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>250,000 API requests/month</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>20 GB storage</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>10 projects</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>5 collaborators</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>100K AI tokens/month</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>Hosted mock deployments</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>Advanced analytics</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>Priority support (48h SLA)</span>
              </li>
            </ul>
            <Button className="w-full" variant="default" onClick={() => handleUpgrade('pro')}>
              {hasTrial ? paidCtaLabel : 'Upgrade to Pro'}
              <ArrowRight className="ml-2 h-4 w-4" />
            </Button>
          </CardContent>
        </Card>

        {/* Team Plan */}
        <Card className="relative">
          <CardHeader>
            <CardTitle className="text-2xl">Team</CardTitle>
            <div className="mt-4">
              <span className="text-4xl font-bold">$99</span>
              <span className="text-muted-foreground">/month</span>
            </div>
            {trialSubtitle && (
              <p className="text-sm text-success-600 font-medium mt-1">{trialSubtitle}</p>
            )}
            <CardDescription>For growing teams</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <ul className="space-y-3">
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>1,000,000 API requests/month</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>100 GB storage</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>Unlimited projects</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>20 collaborators</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>1M AI tokens/month</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>Hosted mock deployments</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>SSO support</span>
              </li>
              <li className="flex items-start">
                <CheckCircle2 className="w-5 h-5 mr-2 text-success-500 flex-shrink-0 mt-0.5" />
                <span>Dedicated support (24h SLA)</span>
              </li>
            </ul>
            <Button className="w-full" variant="outline" onClick={() => handleUpgrade('team')}>
              {hasTrial ? paidCtaLabel : 'Upgrade to Team'}
              <ArrowRight className="ml-2 h-4 w-4" />
            </Button>
          </CardContent>
        </Card>
      </div>

      {/* Feature Comparison Table */}
      <Card className="mb-12">
        <CardHeader>
          <CardTitle>Feature Comparison</CardTitle>
          <CardDescription>Compare plans side-by-side</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b">
                  <th className="text-left p-4">Feature</th>
                  <th className="text-center p-4">Free</th>
                  <th className="text-center p-4">Pro</th>
                  <th className="text-center p-4">Team</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b">
                  <td className="p-4 font-medium">Monthly Requests</td>
                  <td className="text-center p-4">10,000</td>
                  <td className="text-center p-4">250,000</td>
                  <td className="text-center p-4">1,000,000</td>
                </tr>
                <tr className="border-b">
                  <td className="p-4 font-medium">Storage</td>
                  <td className="text-center p-4">1 GB</td>
                  <td className="text-center p-4">20 GB</td>
                  <td className="text-center p-4">100 GB</td>
                </tr>
                <tr className="border-b">
                  <td className="p-4 font-medium">Projects</td>
                  <td className="text-center p-4">1</td>
                  <td className="text-center p-4">10</td>
                  <td className="text-center p-4">Unlimited</td>
                </tr>
                <tr className="border-b">
                  <td className="p-4 font-medium">Collaborators</td>
                  <td className="text-center p-4">1</td>
                  <td className="text-center p-4">5</td>
                  <td className="text-center p-4">20</td>
                </tr>
                <tr className="border-b">
                  <td className="p-4 font-medium">Hosted Mocks</td>
                  <td className="text-center p-4">
                    <XCircle className="w-5 h-5 mx-auto text-muted-foreground" />
                  </td>
                  <td className="text-center p-4">
                    <CheckCircle2 className="w-5 h-5 mx-auto text-success-500" />
                  </td>
                  <td className="text-center p-4">
                    <CheckCircle2 className="w-5 h-5 mx-auto text-success-500" />
                  </td>
                </tr>
                <tr className="border-b">
                  <td className="p-4 font-medium">AI Tokens (Included)</td>
                  <td className="text-center p-4">BYOK only</td>
                  <td className="text-center p-4">100K</td>
                  <td className="text-center p-4">1M</td>
                </tr>
                <tr className="border-b">
                  <td className="p-4 font-medium">SSO Support</td>
                  <td className="text-center p-4">
                    <XCircle className="w-5 h-5 mx-auto text-muted-foreground" />
                  </td>
                  <td className="text-center p-4">
                    <XCircle className="w-5 h-5 mx-auto text-muted-foreground" />
                  </td>
                  <td className="text-center p-4">
                    <CheckCircle2 className="w-5 h-5 mx-auto text-success-500" />
                  </td>
                </tr>
                <tr>
                  <td className="p-4 font-medium">Support SLA</td>
                  <td className="text-center p-4">Best effort</td>
                  <td className="text-center p-4">48 hours</td>
                  <td className="text-center p-4">24 hours</td>
                </tr>
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>

      {/* FAQ Section */}
      <Card>
        <CardHeader>
          <CardTitle>Frequently Asked Questions</CardTitle>
        </CardHeader>
        <CardContent className="space-y-6">
          <div>
            <h3 className="font-semibold mb-2">Can I change plans later?</h3>
            <p className="text-muted-foreground">
              Yes! You can upgrade or downgrade at any time. Upgrades take effect immediately with prorated billing.
            </p>
          </div>
          <div>
            <h3 className="font-semibold mb-2">What happens if I exceed my limits?</h3>
            <p className="text-muted-foreground">
              You'll receive warnings at 80% and 95% usage. Requests are rate-limited when you hit your limit. Upgrade to increase limits.
            </p>
          </div>
          <div>
            <h3 className="font-semibold mb-2">Do unused requests roll over?</h3>
            <p className="text-muted-foreground">
              No, limits reset each billing cycle. Unused capacity does not roll over to the next month.
            </p>
          </div>
          <div>
            <h3 className="font-semibold mb-2">Can I get a refund?</h3>
            <p className="text-muted-foreground">
              Yes! We offer a 14-day money-back guarantee for Pro and Team plans. Contact support@mockforge.dev for refunds.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* CTA Section */}
      <div className="text-center mt-12">
        <h2 className="text-2xl font-bold mb-4">Ready to get started?</h2>
        <p className="text-muted-foreground mb-6">
          Start with the Free plan. No credit card required.
        </p>
        <Button size="lg" onClick={handleGetStarted}>
          Get Started Free
          <ArrowRight className="ml-2 h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
