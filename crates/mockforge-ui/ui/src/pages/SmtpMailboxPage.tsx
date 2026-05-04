import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/Badge';
import { Mail, Search, Trash2, RefreshCw, Inbox, AlertCircle } from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';
import { authenticatedFetch } from '@/utils/apiClient';

interface SmtpEmail {
  id: string;
  from: string;
  to: string[];
  subject: string;
  body?: string;
  html_body?: string;
  received_at: string;
  size?: number;
}

const API_BASE = '/__mockforge/api/smtp/mailbox';

async function apiFetch<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await authenticatedFetch(url, init);
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    if (response.status === 501) {
      throw new Error(body.message || 'SMTP server is not enabled on this instance');
    }
    throw new Error(body.message || body.error || `Request failed (${response.status})`);
  }
  return response.json() as Promise<T>;
}

const fetchEmails = () => apiFetch<SmtpEmail[]>(API_BASE);
const searchEmails = (params: { sender?: string; recipient?: string; subject?: string; body?: string }) => {
  const qs = new URLSearchParams();
  Object.entries(params).forEach(([k, v]) => v && qs.set(k, v));
  return apiFetch<SmtpEmail[]>(`${API_BASE}/search?${qs}`);
};
const getEmail = (id: string) => apiFetch<SmtpEmail>(`${API_BASE}/${id}`);
const clearMailbox = () => apiFetch<{ message: string }>(API_BASE, { method: 'DELETE' });

export function SmtpMailboxPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [searchFilters, setSearchFilters] = useState({
    sender: '',
    recipient: '',
    subject: '',
    body: '',
  });
  const [activeFilters, setActiveFilters] = useState(searchFilters);

  const hasActiveFilters = Object.values(activeFilters).some(Boolean);

  const { data: emails, isLoading, error, refetch } = useQuery({
    queryKey: ['smtp-emails', activeFilters],
    queryFn: () => (hasActiveFilters ? searchEmails(activeFilters) : fetchEmails()),
  });

  const { data: selectedEmail } = useQuery({
    queryKey: ['smtp-email', selectedId],
    queryFn: () => (selectedId ? getEmail(selectedId) : Promise.resolve(null)),
    enabled: !!selectedId,
  });

  const clearMutation = useMutation({
    mutationFn: clearMailbox,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['smtp-emails'] });
      setSelectedId(null);
      showToast('Mailbox cleared', 'success');
    },
    onError: (err: Error) => showToast(err.message, 'error'),
  });

  const applySearch = (e: React.FormEvent) => {
    e.preventDefault();
    setActiveFilters(searchFilters);
  };

  const resetSearch = () => {
    const empty = { sender: '', recipient: '', subject: '', body: '' };
    setSearchFilters(empty);
    setActiveFilters(empty);
  };

  return (
    <div className="container mx-auto px-4 py-6 max-w-7xl">
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Mail className="h-6 w-6" />
            SMTP Mailbox
          </h1>
          <p className="text-sm text-muted-foreground">
            View and manage emails captured by the mock SMTP server.
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="h-4 w-4 mr-1" />
            Refresh
          </Button>
          <Button
            variant="destructive"
            size="sm"
            disabled={clearMutation.isPending || !emails?.length}
            onClick={() => {
              if (confirm('Clear all emails from the mailbox?')) {
                clearMutation.mutate();
              }
            }}
          >
            <Trash2 className="h-4 w-4 mr-1" />
            Clear mailbox
          </Button>
        </div>
      </div>

      {error && (
        <Card className="mb-4 border-danger-200 bg-danger-50 dark:bg-danger-900/10 dark:border-danger-800">
          <CardContent className="pt-4 flex items-start gap-2">
            <AlertCircle className="h-5 w-5 text-danger-600 flex-shrink-0 mt-0.5" />
            <p className="text-sm text-danger-700 dark:text-danger-200">
              {error instanceof Error ? error.message : 'Failed to load mailbox'}
            </p>
          </CardContent>
        </Card>
      )}

      <Card className="mb-4">
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <Search className="h-4 w-4" />
            Search
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={applySearch} className="grid grid-cols-1 md:grid-cols-4 gap-3">
            <Input
              placeholder="From"
              value={searchFilters.sender}
              onChange={(e) => setSearchFilters({ ...searchFilters, sender: e.target.value })}
            />
            <Input
              placeholder="To"
              value={searchFilters.recipient}
              onChange={(e) => setSearchFilters({ ...searchFilters, recipient: e.target.value })}
            />
            <Input
              placeholder="Subject"
              value={searchFilters.subject}
              onChange={(e) => setSearchFilters({ ...searchFilters, subject: e.target.value })}
            />
            <Input
              placeholder="Body contains…"
              value={searchFilters.body}
              onChange={(e) => setSearchFilters({ ...searchFilters, body: e.target.value })}
            />
            <div className="md:col-span-4 flex justify-end gap-2">
              {hasActiveFilters && (
                <Button type="button" variant="outline" size="sm" onClick={resetSearch}>
                  Clear filters
                </Button>
              )}
              <Button type="submit" size="sm">Search</Button>
            </div>
          </form>
        </CardContent>
      </Card>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <Card className="lg:col-span-1">
          <CardHeader>
            <CardTitle className="text-base">Inbox</CardTitle>
            <CardDescription>
              {isLoading ? 'Loading…' : `${emails?.length ?? 0} email${emails?.length === 1 ? '' : 's'}`}
            </CardDescription>
          </CardHeader>
          <CardContent className="p-0">
            {isLoading ? (
              <div className="p-4 text-center text-muted-foreground">Loading…</div>
            ) : !emails || emails.length === 0 ? (
              <div className="p-8 text-center text-muted-foreground">
                <Inbox className="h-10 w-10 mx-auto mb-2 opacity-50" />
                No emails
              </div>
            ) : (
              <div className="max-h-[600px] overflow-y-auto divide-y">
                {emails.map((email) => (
                  <button
                    key={email.id}
                    onClick={() => setSelectedId(email.id)}
                    className={`w-full text-left p-3 hover:bg-muted/50 transition-colors ${
                      selectedId === email.id ? 'bg-muted' : ''
                    }`}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-sm font-medium truncate max-w-[180px]">{email.from}</span>
                      <span className="text-xs text-muted-foreground">
                        {new Date(email.received_at).toLocaleTimeString()}
                      </span>
                    </div>
                    <p className="text-sm truncate">{email.subject || '(no subject)'}</p>
                    <p className="text-xs text-muted-foreground truncate">
                      to: {email.to.join(', ')}
                    </p>
                  </button>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle className="text-base">
              {selectedEmail ? selectedEmail.subject || '(no subject)' : 'Select an email'}
            </CardTitle>
            {selectedEmail && (
              <CardDescription>
                <div className="flex flex-wrap gap-2 mt-1 text-xs">
                  <Badge variant="secondary">From: {selectedEmail.from}</Badge>
                  <Badge variant="secondary">To: {selectedEmail.to.join(', ')}</Badge>
                  <Badge variant="outline">{new Date(selectedEmail.received_at).toLocaleString()}</Badge>
                  {selectedEmail.size && <Badge variant="outline">{selectedEmail.size} bytes</Badge>}
                </div>
              </CardDescription>
            )}
          </CardHeader>
          <CardContent>
            {!selectedEmail ? (
              <p className="text-muted-foreground text-sm text-center py-8">
                Select an email from the list to view its content.
              </p>
            ) : selectedEmail.html_body ? (
              <iframe
                srcDoc={selectedEmail.html_body}
                title="Email body"
                className="w-full min-h-[400px] border rounded"
                sandbox=""
              />
            ) : (
              <pre className="text-sm whitespace-pre-wrap font-mono bg-muted/30 p-3 rounded max-h-[500px] overflow-auto">
                {selectedEmail.body || '(empty)'}
              </pre>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
