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
import { Plus, RefreshCw, Trash2, Copy, ExternalLink, X } from 'lucide-react';
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
  const [selectedCapture, setSelectedCapture] = useState<CloudProxyCapture | null>(null);
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
            <CaptureBrowser
              sessionId={selectedSessionId}
              onSelect={setSelectedCapture}
            />
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

      {selectedCapture && (
        <CaptureDetailDialog
          capture={selectedCapture}
          onClose={() => setSelectedCapture(null)}
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

interface CaptureBrowserProps {
  sessionId: string;
  onSelect: (capture: CloudProxyCapture) => void;
}

const CaptureBrowser: React.FC<CaptureBrowserProps> = ({ sessionId, onSelect }) => {
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
          <CaptureRow key={c.id} capture={c} onSelect={onSelect} />
        ))}
      </ul>
    </div>
  );
};

interface CaptureRowProps {
  capture: CloudProxyCapture;
  onSelect: (capture: CloudProxyCapture) => void;
}

const CaptureRow: React.FC<CaptureRowProps> = ({ capture, onSelect }) => {
  const status = capture.response_status;
  const statusClass = !status
    ? 'text-gray-500'
    : status >= 500
      ? 'text-red-600 dark:text-red-400'
      : status >= 400
        ? 'text-amber-600 dark:text-amber-400'
        : 'text-green-700 dark:text-green-400';
  return (
    <li
      className="px-4 py-2 grid grid-cols-12 gap-2 text-sm hover:bg-gray-50 dark:hover:bg-gray-800 cursor-pointer"
      onClick={() => onSelect(capture)}
    >
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

const CaptureDetailDialog: React.FC<{
  capture: CloudProxyCapture;
  onClose: () => void;
}> = ({ capture, onClose }) => {
  return (
    <div
      className="fixed inset-0 bg-black/40 flex items-center justify-center z-50 p-4"
      onClick={onClose}
    >
      <div
        className="bg-white dark:bg-gray-900 rounded-lg shadow-xl w-full max-w-4xl max-h-[90vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-start justify-between gap-2 p-4 border-b border-gray-200 dark:border-gray-700">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <code className="font-mono text-sm font-semibold text-gray-900 dark:text-gray-100">
                {capture.method}
              </code>
              <code className="font-mono text-sm text-gray-700 dark:text-gray-300 truncate">
                {capture.path}
                {capture.query_string ? `?${capture.query_string}` : ''}
              </code>
            </div>
            <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              {new Date(capture.occurred_at).toLocaleString()} ·{' '}
              {capture.duration_ms.toLocaleString()} ms
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1 text-gray-500 hover:text-gray-900 dark:hover:text-gray-100"
            aria-label="Close detail"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="overflow-y-auto p-4 space-y-6 flex-1">
          {capture.upstream_error && (
            <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-800 dark:text-red-300 text-sm px-3 py-2 rounded">
              <div className="font-semibold mb-1">Upstream error</div>
              <code className="font-mono text-xs whitespace-pre-wrap break-all">
                {capture.upstream_error}
              </code>
            </div>
          )}

          <CaptureSection title="Request">
            <DetailRow label="Size">
              {capture.request_size_bytes.toLocaleString()} bytes
              {capture.request_body_truncated && (
                <span className="ml-2 text-amber-600 dark:text-amber-400">
                  (truncated to 1 MB)
                </span>
              )}
            </DetailRow>
            <DetailRow label="Headers">
              <HeaderTable raw={capture.request_headers} />
            </DetailRow>
            <DetailRow label="Body">
              <BodyView
                content={capture.request_body}
                encoding={capture.request_body_encoding}
                truncated={capture.request_body_truncated}
              />
            </DetailRow>
          </CaptureSection>

          <CaptureSection title="Response">
            <DetailRow label="Status">
              <StatusBadge status={capture.response_status} />
            </DetailRow>
            <DetailRow label="Size">
              {capture.response_size_bytes !== null
                ? `${capture.response_size_bytes.toLocaleString()} bytes`
                : '—'}
              {capture.response_body_truncated && (
                <span className="ml-2 text-amber-600 dark:text-amber-400">
                  (truncated to 1 MB)
                </span>
              )}
            </DetailRow>
            <DetailRow label="Headers">
              <HeaderTable raw={capture.response_headers} />
            </DetailRow>
            <DetailRow label="Body">
              <BodyView
                content={capture.response_body}
                encoding={capture.response_body_encoding}
                truncated={capture.response_body_truncated}
              />
            </DetailRow>
          </CaptureSection>
        </div>
      </div>
    </div>
  );
};

const CaptureSection: React.FC<{ title: string; children: React.ReactNode }> = ({
  title,
  children,
}) => (
  <section>
    <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-2">{title}</h3>
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
      {children}
    </div>
  </section>
);

const DetailRow: React.FC<{ label: string; children: React.ReactNode }> = ({
  label,
  children,
}) => (
  <div className="grid grid-cols-12 gap-2 px-3 py-2 border-b last:border-b-0 border-gray-200 dark:border-gray-700">
    <div className="col-span-3 text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">
      {label}
    </div>
    <div className="col-span-9 text-sm text-gray-900 dark:text-gray-100 min-w-0">
      {children}
    </div>
  </div>
);

const StatusBadge: React.FC<{ status: number | null }> = ({ status }) => {
  if (status === null) return <span className="text-gray-500">—</span>;
  const cls =
    status >= 500
      ? 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300'
      : status >= 400
        ? 'bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-300'
        : status >= 300
          ? 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300'
          : 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300';
  return (
    <span className={`inline-block px-2 py-0.5 rounded text-xs font-mono font-semibold ${cls}`}>
      {status}
    </span>
  );
};

const HeaderTable: React.FC<{ raw: string | null }> = ({ raw }) => {
  if (!raw) {
    return <span className="text-xs text-gray-500 dark:text-gray-400">(none)</span>;
  }
  let parsed: Record<string, string>;
  try {
    parsed = JSON.parse(raw) as Record<string, string>;
  } catch {
    return (
      <code className="font-mono text-xs whitespace-pre-wrap text-gray-700 dark:text-gray-300">
        {raw}
      </code>
    );
  }
  const entries = Object.entries(parsed);
  if (entries.length === 0) {
    return <span className="text-xs text-gray-500 dark:text-gray-400">(none)</span>;
  }
  return (
    <dl className="font-mono text-xs space-y-0.5">
      {entries.map(([name, value]) => (
        <div key={name} className="grid grid-cols-12 gap-2">
          <dt className="col-span-4 text-gray-600 dark:text-gray-400 truncate">{name}:</dt>
          <dd className="col-span-8 text-gray-800 dark:text-gray-200 break-all">{value}</dd>
        </div>
      ))}
    </dl>
  );
};

const BodyView: React.FC<{
  content: string | null;
  encoding: string | null;
  truncated: boolean;
}> = ({ content, encoding, truncated }) => {
  if (!content) {
    return <span className="text-xs text-gray-500 dark:text-gray-400">(empty)</span>;
  }
  const isBase64 = encoding === 'base64';
  return (
    <div>
      {isBase64 && (
        <div className="text-xs text-amber-700 dark:text-amber-400 mb-1">
          Non-UTF-8 body — shown as base64.
        </div>
      )}
      <pre className="font-mono text-xs whitespace-pre-wrap break-all bg-gray-50 dark:bg-gray-800 rounded p-2 max-h-64 overflow-auto text-gray-800 dark:text-gray-200">
        {content}
      </pre>
      {truncated && (
        <div className="text-xs text-amber-600 dark:text-amber-400 mt-1">
          Body was truncated at 1 MB; the full payload was not persisted.
        </div>
      )}
    </div>
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
