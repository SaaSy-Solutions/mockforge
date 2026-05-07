/**
 * Cloud Behavioral Cloning page (#393).
 *
 * Sibling of the local BehavioralCloningPage (Flows / Scenarios). The cloud
 * surface is built on different primitives — capture-sessions group recorded
 * exchanges and clone-models are the trained replay artifacts — so this page
 * is clone-model-centric rather than scenario-centric.
 *
 * Differs from CloudRecorderPage (which focuses on session management):
 *   - Live SSE streaming of training and replay test_run events.
 *   - Clone-model timeline view (training progress, runner seconds, metrics).
 *   - Preview banner: the registry's CloneTrainExecutor + ReplayExecutor
 *     still synthesize events until real workers ship (per
 *     `handlers/captures.rs:235-238`), so any queued runs produce simulated
 *     output.
 */
import React, { useEffect, useRef, useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Activity,
  AlertTriangle,
  Brain,
  PlayCircle,
  RefreshCw,
  Sparkles,
  Trash2,
  X,
} from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import {
  cloudRecorderApi,
  type CaptureSession,
  type CloneModel,
} from '../services/api/cloudRecorder';
import { cloudTestRunsApi, type TestRun } from '../services/api/cloudTestRuns';

interface StreamEvent {
  id: number;
  type: string;
  data: unknown;
  receivedAt: string;
}

export const CloudBehavioralCloningPage: React.FC = () => {
  if (!isCloudMode()) {
    return (
      <div className="p-6 max-w-7xl mx-auto">
        <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
          The cloud Behavioral Cloning surface uses workspace-scoped
          capture-sessions and trained clone-models. In self-hosted mode use
          the local Flows / Scenarios page.
        </div>
      </div>
    );
  }
  return <CloudView />;
};

const CloudView: React.FC = () => {
  const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);
  const queryClient = useQueryClient();
  const [trainTarget, setTrainTarget] = useState<CaptureSession | null>(null);
  const [trainName, setTrainName] = useState('');
  const [replayTarget, setReplayTarget] = useState<CaptureSession | null>(null);
  const [replayUrl, setReplayUrl] = useState('');
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [streamSourceRunId, setStreamSourceRunId] = useState<string | null>(null);
  const [streamLabel, setStreamLabel] = useState<string | null>(null);

  const workspaceId = activeWorkspace?.id;

  const sessionsQuery = useQuery({
    queryKey: ['cloud', 'bc', 'sessions', workspaceId],
    queryFn: () => cloudRecorderApi.listSessions(workspaceId!),
    enabled: !!workspaceId,
  });

  const clonesQuery = useQuery({
    queryKey: ['cloud', 'bc', 'clones', workspaceId],
    queryFn: () => cloudRecorderApi.listCloneModels(workspaceId!),
    enabled: !!workspaceId,
    refetchInterval: 10_000,
  });

  const trainMutation = useMutation({
    mutationFn: () =>
      cloudRecorderApi.trainClone(trainTarget!.id, { name: trainName }),
    onSuccess: async (clone) => {
      setActionMessage(
        `Training queued — clone ${clone.id.slice(0, 8)} (${clone.status}).`,
      );
      setTrainTarget(null);
      setTrainName('');
      queryClient.invalidateQueries({
        queryKey: ['cloud', 'bc', 'clones', workspaceId],
      });
      // Resolve the test_run that the train endpoint enqueued (suite_id = clone.id)
      // so we can attach the live SSE stream.
      try {
        const runs = await cloudTestRunsApi.listSuiteRuns(clone.id, 1);
        const run = runs[0];
        if (run) {
          setStreamSourceRunId(run.id);
          setStreamLabel(`Training ${clone.name}`);
        }
      } catch (err) {
        // Stream is best-effort; the clone status still updates via polling.
        console.warn('Failed to fetch training run id for streaming', err);
      }
    },
    onError: (err: Error) => setActionMessage(`Train failed: ${err.message}`),
  });

  const replayMutation = useMutation({
    mutationFn: () =>
      cloudRecorderApi.replaySession(replayTarget!.id, {
        target_url: replayUrl || undefined,
      }),
    onSuccess: (run) => {
      setActionMessage(`Replay queued — run ${run.id.slice(0, 8)}.`);
      setReplayTarget(null);
      setReplayUrl('');
      setStreamSourceRunId(run.id);
      setStreamLabel(`Replay run ${run.id.slice(0, 8)}`);
    },
    onError: (err: Error) => setActionMessage(`Replay failed: ${err.message}`),
  });

  const deleteCloneMutation = useMutation({
    mutationFn: (id: string) => cloudRecorderApi.deleteCloneModel(id),
    onSuccess: () =>
      queryClient.invalidateQueries({
        queryKey: ['cloud', 'bc', 'clones', workspaceId],
      }),
  });

  const openCloneStream = async (clone: CloneModel) => {
    try {
      const runs = await cloudTestRunsApi.listSuiteRuns(clone.id, 1);
      const run = runs[0];
      if (!run) {
        setActionMessage(`No training run recorded for ${clone.name} yet.`);
        return;
      }
      setStreamSourceRunId(run.id);
      setStreamLabel(`Training ${clone.name}`);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'unknown error';
      setActionMessage(`Couldn't open stream: ${message}`);
    }
  };

  if (!workspaceId) {
    return (
      <div className="p-6 max-w-7xl mx-auto">
        <div className="bg-yellow-50 dark:bg-yellow-900/20 text-yellow-800 dark:text-yellow-300 p-4 rounded-lg">
          Select a workspace to manage capture sessions and clone models.
        </div>
      </div>
    );
  }

  const sessions = sessionsQuery.data ?? [];
  const clones = clonesQuery.data ?? [];

  return (
    <div className="p-6 max-w-7xl mx-auto">
      <div className="flex justify-between items-start mb-4">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2 flex items-center gap-2">
            <Brain className="w-6 h-6 text-purple-500" />
            Behavioral Cloning
          </h1>
          <p className="text-gray-600 dark:text-gray-400 max-w-3xl">
            Train deterministic replay clones from grouped capture sessions.
            Equivalent of the local Flows / Scenarios workflow against the
            cloud's <code>capture-sessions</code> + <code>clone-models</code>{' '}
            primitives.
          </p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => {
              sessionsQuery.refetch();
              clonesQuery.refetch();
            }}
            className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
          >
            <RefreshCw
              className={`w-4 h-4 mr-2 ${
                sessionsQuery.isFetching || clonesQuery.isFetching ? 'animate-spin' : ''
              }`}
            />
            Refresh
          </button>
        </div>
      </div>

      <div className="mb-6 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-900/40 text-amber-800 dark:text-amber-300 p-4 rounded-lg flex items-start gap-3">
        <AlertTriangle className="w-5 h-5 flex-shrink-0 mt-0.5" />
        <div className="text-sm">
          <strong>Preview:</strong> The registry's clone training and replay
          executors emit <em>synthetic</em> per-capture events until the real
          workers ship. Status transitions, runner seconds, and event streams
          are simulated — useful for wiring UI and webhooks today, not for
          recording production behavior.
        </div>
      </div>

      {actionMessage && (
        <div className="mb-4 bg-green-50 dark:bg-green-900/20 text-green-800 dark:text-green-400 p-4 rounded-lg text-sm flex items-center justify-between">
          <span>{actionMessage}</span>
          <button onClick={() => setActionMessage(null)} className="text-xs underline">
            dismiss
          </button>
        </div>
      )}

      <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
        <Sparkles className="w-4 h-4 text-purple-500" />
        Trained Clone Models
      </h2>
      {clones.length === 0 && !clonesQuery.isLoading ? (
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-8 text-center mb-8">
          <Brain className="w-12 h-12 mx-auto text-gray-400 mb-3" />
          <h3 className="text-base font-medium text-gray-900 dark:text-gray-100 mb-2">
            No clone models yet
          </h3>
          <p className="text-gray-500 dark:text-gray-400 text-sm">
            Pick a capture session below and queue a training run.
          </p>
        </div>
      ) : (
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden mb-8">
          <table className="w-full text-left text-sm">
            <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
              <tr>
                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Status</th>
                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Source</th>
                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Runner</th>
                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Created</th>
                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
              {clones.map((c) => (
                <CloneRow
                  key={c.id}
                  clone={c}
                  sourceLabel={
                    sessions.find((s) => s.id === c.source_session_id)?.name ?? c.source_session_id?.slice(0, 8) ?? '—'
                  }
                  onWatch={() => openCloneStream(c)}
                  onDelete={() => {
                    if (confirm(`Delete clone "${c.name}"?`)) {
                      deleteCloneMutation.mutate(c.id);
                    }
                  }}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}

      <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
        Capture Sessions
      </h2>
      <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
        Sessions are managed on the{' '}
        <a href="/cloud-recorder" className="text-cyan-600 hover:underline">
          Cloud Recorder
        </a>{' '}
        page. From here, train a clone or replay a session against a target.
      </p>
      {sessions.length === 0 && !sessionsQuery.isLoading ? (
        <div className="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-6 text-sm text-gray-500 dark:text-gray-400 italic">
          No capture sessions in this workspace. Create one on the Cloud
          Recorder page.
        </div>
      ) : (
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
          <table className="w-full text-left text-sm">
            <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
              <tr>
                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Created</th>
                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
              {sessions.map((s) => (
                <SessionRow
                  key={s.id}
                  session={s}
                  onTrain={() => {
                    setTrainTarget(s);
                    setTrainName(`${s.name}-clone`);
                  }}
                  onReplay={() => setReplayTarget(s)}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}

      {trainTarget && (
        <TrainModal
          session={trainTarget}
          name={trainName}
          setName={setTrainName}
          onClose={() => setTrainTarget(null)}
          onSubmit={() => trainMutation.mutate()}
          submitting={trainMutation.isPending}
          error={trainMutation.error ? (trainMutation.error as Error).message : null}
        />
      )}

      {replayTarget && (
        <ReplayModal
          session={replayTarget}
          url={replayUrl}
          setUrl={setReplayUrl}
          onClose={() => setReplayTarget(null)}
          onSubmit={() => replayMutation.mutate()}
          submitting={replayMutation.isPending}
          error={replayMutation.error ? (replayMutation.error as Error).message : null}
        />
      )}

      {streamSourceRunId && (
        <RunStreamDrawer
          runId={streamSourceRunId}
          label={streamLabel ?? 'Run'}
          onClose={() => {
            setStreamSourceRunId(null);
            setStreamLabel(null);
          }}
        />
      )}
    </div>
  );
};

const CloneRow: React.FC<{
  clone: CloneModel;
  sourceLabel: string;
  onWatch: () => void;
  onDelete: () => void;
}> = ({ clone, sourceLabel, onWatch, onDelete }) => {
  const styles: Record<string, string> = {
    ready:
      'bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30',
    training:
      'bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-900/20 dark:text-blue-400 dark:border-blue-900/30',
    failed:
      'bg-red-50 text-red-700 border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-900/30',
  };
  return (
    <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
      <td className="px-6 py-4">
        <div className="font-medium text-gray-900 dark:text-gray-100">{clone.name}</div>
        <div className="text-xs text-gray-400 font-mono">{clone.id.slice(0, 8)}</div>
      </td>
      <td className="px-6 py-4">
        <span
          className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${
            styles[clone.status] ??
            'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700'
          }`}
        >
          {clone.status}
        </span>
      </td>
      <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">{sourceLabel}</td>
      <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">
        {clone.runner_seconds != null ? `${clone.runner_seconds}s` : '—'}
      </td>
      <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">
        {new Date(clone.created_at).toLocaleString()}
      </td>
      <td className="px-6 py-4 text-right space-x-1">
        <button
          onClick={onWatch}
          className="p-2 text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg"
          title="Watch live training events"
        >
          <Activity className="w-4 h-4" />
        </button>
        <button
          onClick={onDelete}
          className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
          title="Delete clone"
        >
          <Trash2 className="w-4 h-4" />
        </button>
      </td>
    </tr>
  );
};

const SessionRow: React.FC<{
  session: CaptureSession;
  onTrain: () => void;
  onReplay: () => void;
}> = ({ session, onTrain, onReplay }) => (
  <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
    <td className="px-6 py-4">
      <div className="font-medium text-gray-900 dark:text-gray-100">{session.name}</div>
      {session.description && (
        <div className="text-xs text-gray-500 mt-0.5">{session.description}</div>
      )}
    </td>
    <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">
      {new Date(session.created_at).toLocaleString()}
    </td>
    <td className="px-6 py-4 text-right space-x-1">
      <button
        onClick={onReplay}
        className="p-2 text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg"
        title="Replay against a target URL"
      >
        <PlayCircle className="w-4 h-4" />
      </button>
      <button
        onClick={onTrain}
        className="p-2 text-purple-600 hover:bg-purple-50 dark:hover:bg-purple-900/20 rounded-lg"
        title="Train behavioral clone"
      >
        <Brain className="w-4 h-4" />
      </button>
    </td>
  </tr>
);

const TrainModal: React.FC<{
  session: CaptureSession;
  name: string;
  setName: (s: string) => void;
  onClose: () => void;
  onSubmit: () => void;
  submitting: boolean;
  error: string | null;
}> = ({ session, name, setName, onClose, onSubmit, submitting, error }) => (
  <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
      <div className="p-6 border-b border-gray-200 dark:border-gray-700">
        <h2 className="text-xl font-semibold flex items-center gap-2">
          <Brain className="w-5 h-5 text-purple-500" />
          Train Clone from "{session.name}"
        </h2>
        <p className="text-xs text-gray-500 mt-1">
          Enqueues a behavioral_clone test_run; live events stream in the side
          drawer. Plan limit <code className="font-mono">max_clone_models</code>{' '}
          applies.
        </p>
      </div>
      <div className="p-6 space-y-4">
        {error && (
          <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
            {error}
          </div>
        )}
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Clone model name"
          className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-purple-500"
        />
      </div>
      <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
        <button onClick={onClose} className="px-4 py-2">
          Cancel
        </button>
        <button
          onClick={onSubmit}
          disabled={!name || submitting}
          className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg disabled:opacity-50"
        >
          {submitting ? 'Queueing…' : 'Train Clone'}
        </button>
      </div>
    </div>
  </div>
);

const ReplayModal: React.FC<{
  session: CaptureSession;
  url: string;
  setUrl: (s: string) => void;
  onClose: () => void;
  onSubmit: () => void;
  submitting: boolean;
  error: string | null;
}> = ({ session, url, setUrl, onClose, onSubmit, submitting, error }) => (
  <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
      <div className="p-6 border-b border-gray-200 dark:border-gray-700">
        <h2 className="text-xl font-semibold flex items-center gap-2">
          <PlayCircle className="w-5 h-5 text-blue-500" />
          Replay "{session.name}"
        </h2>
        <p className="text-xs text-gray-500 mt-1">
          Triggers a replay test_run. Synthetic mode (no target URL) emits
          fabricated per-capture events.
        </p>
      </div>
      <div className="p-6 space-y-4">
        {error && (
          <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
            {error}
          </div>
        )}
        <div className="space-y-2">
          <label className="block text-sm font-medium">Target URL (optional)</label>
          <input
            type="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://api.example.com (defaults to synthetic mode)"
            className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 font-mono text-xs"
          />
        </div>
      </div>
      <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
        <button onClick={onClose} className="px-4 py-2">
          Cancel
        </button>
        <button
          onClick={onSubmit}
          disabled={submitting}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50"
        >
          {submitting ? 'Queueing…' : 'Trigger Replay'}
        </button>
      </div>
    </div>
  </div>
);

const RunStreamDrawer: React.FC<{
  runId: string;
  label: string;
  onClose: () => void;
}> = ({ runId, label, onClose }) => {
  const [events, setEvents] = useState<StreamEvent[]>([]);
  const [run, setRun] = useState<TestRun | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [closed, setClosed] = useState(false);
  const counterRef = useRef(0);

  useEffect(() => {
    let cancelled = false;
    cloudTestRunsApi
      .getRun(runId)
      .then((r) => {
        if (!cancelled) setRun(r);
      })
      .catch((err: Error) => {
        if (!cancelled) setError(`Failed to load run: ${err.message}`);
      });

    const source = cloudTestRunsApi.streamRunEvents(runId);
    const handle = (evt: MessageEvent, type: string) => {
      let parsed: unknown = evt.data;
      try {
        parsed = JSON.parse(evt.data);
      } catch {
        // Leave as raw string.
      }
      counterRef.current += 1;
      setEvents((prev) => [
        ...prev,
        {
          id: counterRef.current,
          type,
          data: parsed,
          receivedAt: new Date().toISOString(),
        },
      ]);
    };
    const types = [
      'message',
      'step_start',
      'step_pass',
      'step_fail',
      'step_skip',
      'log',
      'metric',
      'done',
    ];
    const listeners = types.map((t) => {
      const fn = (evt: MessageEvent) => handle(evt, t);
      source.addEventListener(t, fn as EventListener);
      return { type: t, fn };
    });
    source.onerror = () => {
      setClosed(true);
      source.close();
    };
    return () => {
      cancelled = true;
      listeners.forEach(({ type, fn }) =>
        source.removeEventListener(type, fn as EventListener),
      );
      source.close();
    };
  }, [runId]);

  return (
    <div className="fixed inset-0 z-40 flex justify-end">
      <div className="absolute inset-0 bg-black/30" onClick={onClose} />
      <div className="relative bg-white dark:bg-gray-900 w-full max-w-xl h-full shadow-2xl border-l border-gray-200 dark:border-gray-800 flex flex-col">
        <div className="p-4 border-b border-gray-200 dark:border-gray-800 flex items-start justify-between">
          <div>
            <div className="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wide">
              Live run stream
            </div>
            <div className="text-base font-semibold flex items-center gap-2">
              <Activity className="w-4 h-4 text-blue-500" />
              {label}
            </div>
            <div className="text-xs font-mono text-gray-400 mt-0.5">{runId}</div>
            {run && (
              <div className="text-xs text-gray-500 mt-1">
                kind: <code>{run.kind}</code> · status: <code>{run.status}</code>
              </div>
            )}
          </div>
          <button
            onClick={onClose}
            className="p-1 text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-800 rounded"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
        {error && (
          <div className="m-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
            {error}
          </div>
        )}
        <div className="flex-1 overflow-auto p-4 font-mono text-xs space-y-1 bg-gray-50 dark:bg-gray-950">
          {events.length === 0 && !closed && (
            <div className="text-gray-500 italic">
              Waiting for events…
            </div>
          )}
          {events.map((e) => (
            <div
              key={e.id}
              className="border-b border-gray-200 dark:border-gray-800 pb-1 last:border-0"
            >
              <span className="text-blue-600 dark:text-blue-400">{e.type}</span>{' '}
              <span className="text-gray-400">
                {new Date(e.receivedAt).toLocaleTimeString()}
              </span>
              <pre className="whitespace-pre-wrap break-all text-gray-700 dark:text-gray-300">
                {typeof e.data === 'string'
                  ? e.data
                  : JSON.stringify(e.data, null, 2)}
              </pre>
            </div>
          ))}
          {closed && (
            <div className="text-gray-500 italic mt-2">stream closed.</div>
          )}
        </div>
      </div>
    </div>
  );
};
