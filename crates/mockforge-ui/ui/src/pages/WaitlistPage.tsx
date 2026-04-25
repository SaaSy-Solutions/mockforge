import React, { useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Alert } from '@/components/ui/alert';
import { CheckCircle2, Mail, Sparkles } from 'lucide-react';
import { apiErrorMessage } from '@/utils/errorHandling';

interface WaitlistRequest {
  email: string;
  name?: string;
  company?: string;
  use_case?: string;
}

export function WaitlistPage() {
  const [formData, setFormData] = useState<WaitlistRequest>({
    email: '',
    name: '',
    company: '',
    use_case: '',
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);

    try {
      const response = await fetch('/api/v1/waitlist/subscribe', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(formData),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Failed to join waitlist' }));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to join waitlist'));
      }

      setSuccess(true);
      setFormData({ email: '', name: '', company: '', use_case: '' });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to join waitlist');
    } finally {
      setLoading(false);
    }
  };

  if (success) {
    return (
      <div className="container mx-auto px-4 py-12 max-w-2xl">
        <Card>
          <CardContent className="pt-8 pb-8 text-center">
            <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-green-100 dark:bg-green-900/30">
              <CheckCircle2 className="h-8 w-8 text-green-600 dark:text-green-400" />
            </div>
            <h2 className="mb-2 text-2xl font-bold">You're on the list!</h2>
            <p className="mb-6 text-muted-foreground">
              Thanks for joining the MockForge waitlist. We'll email you when your invitation is ready.
            </p>
            <Button variant="outline" onClick={() => setSuccess(false)}>
              Add another email
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-12 max-w-2xl">
      <div className="text-center mb-8">
        <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
          <Sparkles className="h-6 w-6 text-primary" />
        </div>
        <h1 className="text-4xl font-bold mb-3">Join the MockForge waitlist</h1>
        <p className="text-lg text-muted-foreground">
          Be the first to know when we open up access to new features and hosted mocks.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Request early access</CardTitle>
          <CardDescription>
            We'll send your invitation by email as soon as a spot opens up.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {error && (
            <Alert className="mb-4 bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800">
              <span className="text-red-800 dark:text-red-200">{error}</span>
            </Alert>
          )}

          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label htmlFor="email" className="block text-sm font-medium mb-2">
                Email <span className="text-red-500">*</span>
              </label>
              <Input
                id="email"
                type="email"
                required
                value={formData.email}
                onChange={(e) => setFormData({ ...formData, email: e.target.value })}
                placeholder="you@company.com"
              />
            </div>

            <div>
              <label htmlFor="name" className="block text-sm font-medium mb-2">
                Name
              </label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder="Your full name"
              />
            </div>

            <div>
              <label htmlFor="company" className="block text-sm font-medium mb-2">
                Company
              </label>
              <Input
                id="company"
                value={formData.company}
                onChange={(e) => setFormData({ ...formData, company: e.target.value })}
                placeholder="Company name"
              />
            </div>

            <div>
              <label htmlFor="use_case" className="block text-sm font-medium mb-2">
                What are you building?
              </label>
              <textarea
                id="use_case"
                value={formData.use_case}
                onChange={(e) => setFormData({ ...formData, use_case: e.target.value })}
                placeholder="Briefly describe your use case (optional)"
                rows={3}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 resize-y"
              />
            </div>

            <Button type="submit" className="w-full" disabled={loading}>
              {loading ? (
                <>
                  <div className="mr-2 h-4 w-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                  Joining waitlist...
                </>
              ) : (
                <>
                  <Mail className="mr-2 h-4 w-4" />
                  Join waitlist
                </>
              )}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
