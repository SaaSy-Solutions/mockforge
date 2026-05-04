/**
 * Cloud Recorder Proxy — Phase 5 UI.
 *
 * Lets users create forwarding sessions pinned to an upstream URL,
 * copy the proxy URL into their client, and browse the captured
 * request/response history.
 *
 * Wraps cloudProxyApi (`/api/v1/cloud-runs/recorder-proxy/*`).
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Plus, RefreshCw, Trash2, Copy, ExternalLink } from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import {
  cloudProxyApi,
  type CloudProxySession,
  type CloudProxyCapture,
  type SessionWithProxyUrl,
} from '../services/api/cloudProxy';

export const CloudProxyPage: React.FC = () => {
  if (!isCloudMode()) {
    return (
      <div className="p-6 max-w-7xl mx-auto">
        <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
          The cloud recorder proxy only runs in cloud mode (sessions are
          backed by the registry server's Postgres). Self-hosted users
          can run a local proxy via{' '}
          <code className="font-mono text-xs">mockforge-recorder</code> directly.
        </div>
      </div>
    );
  }
  return <CloudProxyView />;
};

const CloudProxyView: React.FC = () => {
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [justCreated, setJustCreated] = useState<SessionWithProxyUrl | null>(null);

  const queryClient = useQueryClient();

  const sessionsQuery = useQuery({
    queryKey: ['cloud-proxy', 'sessions'],
    queryFn: () => cloudProxyApi.listSessions(50),
    refetchInterval: 30_000,
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => cloudProxyApi.deleteSession(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['cloud-proxy', 'sessions'] });
      setSelectedSessionId(null);
    },
  });

  return (
    <div className="p-6 max-w-7xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Cloud Recorder Proxy
          </h1>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            Create a forwarding session, route your client traffic through it, and inspect
            the captures.
          </p>
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => sessionsQuery.refetch()}
            className="px-3 py-2 text-sm border border-gray-200 dark:border-gray-700 rounded-md hover:bg-gray-50 dark:hover:bg-gray-800 flex items-center gap-2"
          >
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
          <button
            type="button"
            onClick={() => setShowCreate(true)}
            className="px-3 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 flex items-center gap-2"
          >
            <Plus className="w-4 h-4" />
            New session
          </button>
        </div>
      </div>

      {justCreated && (
        <JustCreatedBanner
          session={justCreated}
          onDismiss={() => setJustCreated(null)}
        />
      )}

      <div className="grid grid-cols-12 gap-6">
        <div className="col-span-5">
          <SessionList
            sessions={sessionsQuery.data ?? []}
            selectedId={selectedSessionId}
            isLoading={sessionsQuery.isLoading}
            onSelect={setSelectedSessionId}
            onDelete={(id) => deleteMutation.mutate(id)}
          />
        </div>
        <div className="col-span-7">
          {selectedSessionId ? (
            <CaptureBrowser sessionId={selectedSessionId} />
          ) : (
            <div className="border border-gray-200 dark:border-gray-700 rounded-lg p-8 text-center text-gray-500 dark:text-gray-400">
              Select a session to view its captures.
            </div>
          )}
        </div>
      </div>

      {showCreate && (
        <CreateSessionDialog
          onClose={() => setShowCreate(false)}
          onCreated={(s) => {
            setJustCreated(s);
            setShowCreate(false);
            void queryClient.invalidateQueries({ queryKey: ['cloud-proxy', 'sessions'] });
          }}
        />
      )}
    </div>
  );
};

interface SessionListProps {
  sessions: CloudProxySession[];
  selectedId: string | null;
  isLoading: boolean;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
}

const SessionList: React.FC<SessionListProps> = ({
  sessions,
  selectedId,
  isLoading,
  onSelect,
  onDelete,
}) => {
  if (isLoading) {
    return (
      <div className="border border-gray-200 dark:border-gray-700 rounded-lg p-6 text-sm text-gray-500 dark:text-gray-400">
        Loading sessions…
      </div>
    );
  }
  if (sessions.length === 0) {
    return (
      <div className="border border-gray-200 dark:border-gray-700 rounded-lg p-6 text-sm text-gray-500 dark:text-gray-400">
        No active proxy sessions. Create one to start recording.
      </div>
    );
  }
  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
      <ul className="divide-y divide-gray-200 dark:divide-gray-700">
        {sessions.map((s) => {
          const expiresIn = formatExpiresIn(s.expires_at);
          const isActive = s.id === selectedId;
          return (
            <li
              key={s.id}
              className={`p-4 cursor-pointer transition-colors ${
                isActive
                  ? 'bg-blue-50 dark:bg-blue-900/20'
                  : 'hover:bg-gray-50 dark:hover:bg-gray-800'
              }`}
              onClick={() => onSelect(s.id)}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0 flex-1">
                  <div className="font-medium text-gray-900 dark:text-gray-100 truncate">
                    {s.name ?? '(unnamed)'}
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 truncate font-mono">
                    {s.upstream_url}
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                    {s.capture_count.toLocaleString()} captures · {expiresIn}
                  </div>
                </div>
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    if (
                      window.confirm(
                        'Revoke this session? Existing captures stay queryable but the proxy URL stops working immediately.',
                      )
                    ) {
                      onDelete(s.id);
                    }
                  }}
                  className="p-1 text-gray-400 hover:text-red-600 dark:hover:text-red-400"
                  aria-label="Revoke session"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
};

const CaptureBrowser: React.FC<{ sessionId: string }> = ({ sessionId }) => {
  const capturesQuery = useQuery({
    queryKey: ['cloud-proxy', 'captures', sessionId],
    queryFn: () => cloudProxyApi.listCaptures(sessionId, 100),
    refetchInterval: 5_000,
  });

  if (capturesQuery.isLoading) {
    return (
      <div className="border border-gray-200 dark:border-gray-700 rounded-lg p-6 text-sm text-gray-500 dark:text-gray-400">
        Loading captures…
      </div>
    );
  }
  const captures = capturesQuery.data ?? [];
  if (captures.length === 0) {
    return (
      <div className="border border-gray-200 dark:border-gray-700 rounded-lg p-6 text-sm text-gray-500 dark:text-gray-400">
        No captures yet. Send traffic through the proxy URL to populate this view.
      </div>
    );
  }
  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
      <div className="px-4 py-2 bg-gray-50 dark:bg-gray-800 text-xs font-medium text-gray-600 dark:text-gray-400 grid grid-cols-12 gap-2">
        <div className="col-span-1">Method</div>
        <div className="col-span-6">Path</div>
        <div className="col-span-2">Status</div>
        <div className="col-span-3 text-right">Latency</div>
      </div>
      <ul className="divide-y divide-gray-200 dark:divide-gray-700">
        {captures.map((c) => (
          <CaptureRow key={c.id} capture={c} />
        ))}
      </ul>
    </div>
  );
};

const CaptureRow: React.FC<{ capture: CloudProxyCapture }> = ({ capture }) => {
  const status = capture.response_status;
  const statusClass = !status
    ? 'text-gray-500'
    : status >= 500
      ? 'text-red-600 dark:text-red-400'
      : status >= 400
        ? 'text-amber-600 dark:text-amber-400'
        : 'text-green-700 dark:text-green-400';
  return (
    <li className="px-4 py-2 grid grid-cols-12 gap-2 text-sm hover:bg-gray-50 dark:hover:bg-gray-800">
      <div className="col-span-1 font-mono text-xs">{capture.method}</div>
      <div className="col-span-6 font-mono text-xs text-gray-700 dark:text-gray-300 truncate">
        {capture.path}
        {capture.query_string ? `?${capture.query_string}` : ''}
      </div>
      <div className={`col-span-2 font-mono text-xs ${statusClass}`}>
        {capture.upstream_error ? 'ERR' : (status?.toString() ?? '—')}
      </div>
      <div className="col-span-3 text-right text-xs text-gray-500 dark:text-gray-400">
        {capture.duration_ms.toLocaleString()} ms
      </div>
    </li>
  );
};

interface CreateDialogProps {
  onClose: () => void;
  onCreated: (s: SessionWithProxyUrl) => void;
}

const CreateSessionDialog: React.FC<CreateDialogProps> = ({ onClose, onCreated }) => {
  const [upstreamUrl, setUpstreamUrl] = useState('https://');
  const [name, setName] = useState('');
  const [ttlHours, setTtlHours] = useState(24);
  const [error, setError] = useState<string | null>(null);

  const mutation = useMutation({
    mutationFn: () =>
      cloudProxyApi.createSession({
        upstream_url: upstreamUrl.trim(),
        name: name.trim() || undefined,
        ttl_hours: ttlHours,
      }),
    onSuccess: onCreated,
    onError: (err: Error) => setError(err.message),
  });

  return (
    <div
      className="fixed inset-0 bg-black/40 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <div
        className="bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 w-full max-w-md"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
          New proxy session
        </h2>
        <div className="space-y-4">
          <div>
            <label
              htmlFor="upstream-url"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
            >
              Upstream URL
            </label>
            <input
              id="upstream-url"
              type="text"
              value={upstreamUrl}
              onChange={(e) => setUpstreamUrl(e.target.value)}
              placeholder="https://api.example.com"
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm font-mono text-gray-900 dark:text-gray-100"
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Must be publicly reachable. Internal IPs are rejected.
            </p>
          </div>
          <div>
            <label
              htmlFor="session-name"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
            >
              Name (optional)
            </label>
            <input
              id="session-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="staging-api"
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100"
            />
          </div>
          <div>
            <label
              htmlFor="ttl-hours"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
            >
              Lifetime (hours)
            </label>
            <input
              id="ttl-hours"
              type="number"
              min={1}
              max={168}
              value={ttlHours}
              onChange={(e) => setTtlHours(Number(e.target.value))}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100"
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Session auto-expires after this. Capped at 168 (1 week).
            </p>
          </div>
          {error && (
            <div className="text-sm text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 px-3 py-2 rounded">
              {error}
            </div>
          )}
        </div>
        <div className="flex justify-end gap-2 mt-6">
          <button
            type="button"
            onClick={onClose}
            className="px-3 py-2 text-sm border border-gray-200 dark:border-gray-700 rounded-md hover:bg-gray-50 dark:hover:bg-gray-800"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => {
              setError(null);
              mutation.mutate();
            }}
            disabled={mutation.isPending || !upstreamUrl.trim()}
            className="px-3 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            {mutation.isPending ? 'Creating…' : 'Create'}
          </button>
        </div>
      </div>
    </div>
  );
};

const JustCreatedBanner: React.FC<{
  session: SessionWithProxyUrl;
  onDismiss: () => void;
}> = ({ session, onDismiss }) => {
  const fullUrl = `${window.location.origin}${session.proxy_path}`;
  return (
    <div className="mb-6 border border-green-300 dark:border-green-700 bg-green-50 dark:bg-green-900/20 rounded-lg p-4">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <div className="font-medium text-green-900 dark:text-green-200">
            Session created — copy the proxy URL now
          </div>
          <p className="text-xs text-green-800 dark:text-green-300 mt-1">
            The token in this URL is the only auth — anyone with it can drive the
            proxy. We won't show it again, but you can find the token in the session
            list.
          </p>
          <div className="mt-2 flex items-center gap-2">
            <code className="font-mono text-xs bg-white dark:bg-gray-900 px-2 py-1 rounded border border-green-300 dark:border-green-700 break-all flex-1">
              {fullUrl}
            </code>
            <button
              type="button"
              onClick={() => {
                void navigator.clipboard.writeText(fullUrl);
              }}
              className="p-2 border border-green-300 dark:border-green-700 rounded hover:bg-green-100 dark:hover:bg-green-900/40"
              aria-label="Copy proxy URL"
            >
              <Copy className="w-4 h-4" />
            </button>
            <a
              href={fullUrl}
              target="_blank"
              rel="noreferrer"
              className="p-2 border border-green-300 dark:border-green-700 rounded hover:bg-green-100 dark:hover:bg-green-900/40"
              aria-label="Open proxy URL in new tab"
            >
              <ExternalLink className="w-4 h-4" />
            </a>
          </div>
        </div>
        <button
          type="button"
          onClick={onDismiss}
          className="text-green-700 dark:text-green-400 hover:text-green-900 dark:hover:text-green-200"
        >
          Dismiss
        </button>
      </div>
    </div>
  );
};

function formatExpiresIn(isoTimestamp: string): string {
  const expiresMs = new Date(isoTimestamp).getTime();
  const remainingMs = expiresMs - Date.now();
  if (remainingMs <= 0) return 'expired';
  const hours = Math.floor(remainingMs / (1000 * 60 * 60));
  if (hours < 1) {
    const minutes = Math.floor(remainingMs / (1000 * 60));
    return `expires in ${minutes}m`;
  }
  if (hours < 48) return `expires in ${hours}h`;
  return `expires in ${Math.floor(hours / 24)}d`;
}
