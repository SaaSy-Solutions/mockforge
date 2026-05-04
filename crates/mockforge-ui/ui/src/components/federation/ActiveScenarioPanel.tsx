/**
 * Active Scenario Panel
 *
 * Shows the federation's currently active scenario (if any) and lets the
 * user activate a new scenario (via inline manifest JSON + per-service
 * overrides) or deactivate the active one.
 */

import React, { useMemo, useState } from 'react';
import {
  type Federation,
  type FederationService,
  type PerServiceActivationState,
  type ServiceScenarioOverride,
  useActivateFederationScenario,
  useActiveFederationScenario,
  useDeactivateFederationScenario,
  useOrgScenarios,
  useReportFederationScenarioState,
} from '../../hooks/useFederation';
import { Card } from '../ui/Card';
import { AlertCircle, CheckCircle, Clock, Play, Square, Zap } from 'lucide-react';

export interface ActiveScenarioPanelProps {
  federation: Federation;
}

const STATE_ICON: Record<PerServiceActivationState['status'], React.ReactNode> = {
  pending: <Clock className="h-4 w-4 text-warning-600 dark:text-warning-400" />,
  applied: <CheckCircle className="h-4 w-4 text-success-600 dark:text-success-400" />,
  failed: <AlertCircle className="h-4 w-4 text-danger-600 dark:text-danger-400" />,
};

export const ActiveScenarioPanel: React.FC<ActiveScenarioPanelProps> = ({ federation }) => {
  const { data: active, isLoading } = useActiveFederationScenario(federation.id);
  const activate = useActivateFederationScenario();
  const deactivate = useDeactivateFederationScenario();
  const reportState = useReportFederationScenarioState();
  const [showActivate, setShowActivate] = useState(false);

  const handleForceState = async (
    serviceName: string,
    status: PerServiceActivationState['status']
  ) => {
    const reason =
      status === 'failed'
        ? prompt(`Mark "${serviceName}" as failed. Optional error message:`) ?? undefined
        : undefined;
    try {
      await reportState.mutateAsync({
        federationId: federation.id,
        report: {
          service_name: serviceName,
          status,
          error: reason || null,
        },
      });
    } catch (err) {
      alert(`Failed to report state: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center justify-center">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-info-600"></div>
        </div>
      </Card>
    );
  }

  const handleDeactivate = async () => {
    if (!confirm('Deactivate the current scenario? Workspaces will revert to defaults.')) return;
    try {
      await deactivate.mutateAsync({ federationId: federation.id });
    } catch (err) {
      alert(`Deactivate failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  if (active) {
    return (
      <Card className="p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-foreground flex items-center gap-2">
            <Zap className="h-5 w-5 text-amber-500" />
            Active Scenario
          </h3>
          <button
            onClick={handleDeactivate}
            disabled={deactivate.isPending}
            className="flex items-center gap-2 px-3 py-1.5 bg-danger-600 text-white rounded-lg hover:bg-destructive/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed text-sm"
          >
            <Square className="h-4 w-4" />
            Deactivate
          </button>
        </div>

        <div className="space-y-3 text-sm">
          <div>
            <span className="text-muted-foreground">Scenario:</span>
            <span className="ml-2 font-medium text-foreground">
              {active.scenario_name}
            </span>
          </div>
          <div>
            <span className="text-muted-foreground">Activated:</span>
            <span className="ml-2 text-foreground">
              {new Date(active.activated_at).toLocaleString()}
            </span>
          </div>

          <div className="pt-2">
            <div className="font-medium text-foreground mb-2">Service status</div>
            <div className="space-y-2">
              {active.per_service_state.map((entry) => (
                <div
                  key={entry.service_name}
                  className="flex items-center gap-3 p-2 bg-muted rounded"
                >
                  {STATE_ICON[entry.status]}
                  <div className="flex-1">
                    <div className="font-medium text-foreground">
                      {entry.service_name}
                    </div>
                    {entry.error && (
                      <div className="text-xs text-danger-600 dark:text-danger-400">{entry.error}</div>
                    )}
                    {entry.last_observed_at && (
                      <div className="text-xs text-muted-foreground">
                        last seen {new Date(entry.last_observed_at).toLocaleTimeString()}
                      </div>
                    )}
                  </div>
                  <span className="text-xs uppercase text-muted-foreground">
                    {entry.status}
                  </span>
                  {/* Manual override — same payload runtime pollers use, for
                      unsticking services when a poller is down. */}
                  <div className="flex items-center gap-1">
                    {entry.status !== 'applied' && (
                      <button
                        type="button"
                        onClick={() => handleForceState(entry.service_name, 'applied')}
                        disabled={reportState.isPending}
                        className="text-xs px-2 py-0.5 rounded border border-success-300 dark:border-success-700 text-success-700 dark:text-success-300 hover:bg-success-50 dark:hover:bg-success-900/40 disabled:opacity-50"
                        title="Force-mark this service as applied"
                      >
                        applied
                      </button>
                    )}
                    {entry.status !== 'failed' && (
                      <button
                        type="button"
                        onClick={() => handleForceState(entry.service_name, 'failed')}
                        disabled={reportState.isPending}
                        className="text-xs px-2 py-0.5 rounded border border-danger-300 dark:border-danger-700 text-danger-700 dark:text-danger-300 hover:bg-danger-50 dark:hover:bg-danger-900/40 disabled:opacity-50"
                        title="Force-mark this service as failed"
                      >
                        failed
                      </button>
                    )}
                    {entry.status !== 'pending' && (
                      <button
                        type="button"
                        onClick={() => handleForceState(entry.service_name, 'pending')}
                        disabled={reportState.isPending}
                        className="text-xs px-2 py-0.5 rounded border border-warning-300 dark:border-warning-700 text-warning-700 dark:text-warning-300 hover:bg-warning-50 dark:hover:bg-warning-900/40 disabled:opacity-50"
                        title="Force-reset this service to pending"
                      >
                        reset
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {Object.keys(active.service_overrides || {}).length > 0 && (
            <div className="pt-2">
              <div className="font-medium text-foreground mb-2">Per-service overrides</div>
              <pre className="p-3 bg-muted rounded text-xs overflow-x-auto text-foreground">
                {JSON.stringify(active.service_overrides, null, 2)}
              </pre>
            </div>
          )}
        </div>
      </Card>
    );
  }

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-foreground flex items-center gap-2">
          <Zap className="h-5 w-5 text-muted-foreground" />
          Active Scenario
        </h3>
        {!showActivate && (
          <button
            onClick={() => setShowActivate(true)}
            className="flex items-center gap-2 px-3 py-1.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors text-sm"
          >
            <Play className="h-4 w-4" />
            Activate Scenario
          </button>
        )}
      </div>

      {!showActivate ? (
        <p className="text-sm text-muted-foreground">
          No scenario is currently active on this federation.
        </p>
      ) : (
        <ActivateScenarioForm
          federation={federation}
          onCancel={() => setShowActivate(false)}
          onActivate={async (req) => {
            try {
              await activate.mutateAsync({
                federationId: federation.id,
                request: {
                  scenario_id: req.scenario_id,
                  scenario_name: req.scenario_name,
                  manifest: req.manifest,
                  service_overrides: req.service_overrides,
                },
              });
              setShowActivate(false);
            } catch (err) {
              alert(
                `Activation failed: ${err instanceof Error ? err.message : 'Unknown error'}`
              );
            }
          }}
          pending={activate.isPending}
        />
      )}
    </Card>
  );
};

interface ActivateScenarioFormProps {
  federation: Federation;
  onActivate: (request: {
    scenario_id?: string;
    scenario_name: string;
    manifest: unknown;
    service_overrides: Record<string, ServiceScenarioOverride>;
  }) => Promise<void>;
  onCancel: () => void;
  pending: boolean;
}

const DEFAULT_MANIFEST = () => ({
  manifest_version: '1.0',
  name: 'inline-scenario',
  version: '0.1.0',
  title: 'Inline scenario',
  description: 'Activated via the federation UI',
  author: 'ui',
  category: 'Other',
  compatibility: { min_version: '0.3.0' },
  files: [],
});

const ActivateScenarioForm: React.FC<ActivateScenarioFormProps> = ({
  federation,
  onActivate,
  onCancel,
  pending,
}) => {
  const [scenarioName, setScenarioName] = useState('inline-scenario');
  const [manifestText, setManifestText] = useState(() =>
    JSON.stringify(DEFAULT_MANIFEST(), null, 2)
  );
  const [overrides, setOverrides] = useState<Record<string, ServiceScenarioOverride>>({});
  const [selectedScenarioId, setSelectedScenarioId] = useState<string>('');

  const services = useMemo<FederationService[]>(() => federation.services || [], [federation]);
  const orgScenarios = useOrgScenarios();

  const handleScenarioPick = (id: string) => {
    setSelectedScenarioId(id);
    if (!id) return;
    const picked = orgScenarios.data?.find((s) => s.id === id);
    if (!picked) return;
    setScenarioName(picked.name);
    setManifestText(JSON.stringify(picked.manifest_json, null, 2));
  };

  const handleSubmit = async () => {
    let manifest: unknown;
    try {
      manifest = JSON.parse(manifestText);
    } catch (err) {
      alert(`Manifest is not valid JSON: ${err instanceof Error ? err.message : 'parse error'}`);
      return;
    }
    await onActivate({
      scenario_id: selectedScenarioId || undefined,
      scenario_name: scenarioName,
      manifest,
      service_overrides: overrides,
    });
  };

  const updateOverride = (
    serviceName: string,
    key: keyof ServiceScenarioOverride,
    value: string
  ) => {
    setOverrides((prev) => {
      const next = { ...prev };
      const current: ServiceScenarioOverride = { ...(next[serviceName] || {}) };

      if (value === '') {
        delete (current as Record<string, unknown>)[key];
      } else if (key === 'chaos_level' || key === 'failure_rate' || key === 'latency_ms') {
        const parsed = Number(value);
        if (Number.isNaN(parsed)) return prev;
        (current as Record<string, unknown>)[key] = parsed;
      } else {
        (current as Record<string, unknown>)[key] = value;
      }

      if (Object.keys(current).length === 0) {
        delete next[serviceName];
      } else {
        next[serviceName] = current;
      }
      return next;
    });
  };

  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-foreground mb-1">
          Pick a saved scenario
        </label>
        <select
          value={selectedScenarioId}
          onChange={(e) => handleScenarioPick(e.target.value)}
          className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground"
          disabled={orgScenarios.isLoading}
        >
          <option value="">
            {orgScenarios.isLoading
              ? 'Loading scenarios…'
              : orgScenarios.data?.length
                ? '— inline manifest —'
                : '— no saved scenarios; edit manifest below —'}
          </option>
          {orgScenarios.data?.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name} (v{s.current_version})
            </option>
          ))}
        </select>
        {orgScenarios.isError && (
          <div className="mt-1 text-xs text-danger-600 dark:text-danger-400">
            Failed to load scenarios: {orgScenarios.error?.message}
          </div>
        )}
      </div>

      <div>
        <label className="block text-sm font-medium text-foreground mb-1">
          Scenario name
        </label>
        <input
          type="text"
          value={scenarioName}
          onChange={(e) => setScenarioName(e.target.value)}
          className="w-full px-3 py-2 border border-border rounded-lg bg-card text-foreground"
        />
      </div>

      <div>
        <label className="block text-sm font-medium text-foreground mb-1">
          Manifest JSON
        </label>
        <textarea
          value={manifestText}
          onChange={(e) => setManifestText(e.target.value)}
          rows={10}
          className="w-full px-3 py-2 font-mono text-xs border border-border rounded-lg bg-card text-foreground"
        />
      </div>

      <div>
        <div className="text-sm font-medium text-foreground mb-2">
          Per-service overrides
        </div>
        {services.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            This federation has no services configured.
          </p>
        ) : (
          <div className="space-y-3">
            {services.map((svc) => (
              <div
                key={svc.name}
                className="p-3 bg-muted rounded border border-border"
              >
                <div className="font-medium text-foreground mb-2">{svc.name}</div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
                  <label className="flex flex-col">
                    <span className="text-muted-foreground">Reality level</span>
                    <select
                      value={overrides[svc.name]?.reality_level || ''}
                      onChange={(e) => updateOverride(svc.name, 'reality_level', e.target.value)}
                      className="mt-1 px-2 py-1 border border-border rounded bg-card text-foreground"
                    >
                      <option value="">(no change)</option>
                      <option value="real">real</option>
                      <option value="mock_v3">mock_v3</option>
                      <option value="blended">blended</option>
                      <option value="chaos_driven">chaos_driven</option>
                    </select>
                  </label>
                  <label className="flex flex-col">
                    <span className="text-muted-foreground">Chaos level (0.0–1.0)</span>
                    <input
                      type="number"
                      step="0.1"
                      min="0"
                      max="1"
                      value={overrides[svc.name]?.chaos_level ?? ''}
                      onChange={(e) => updateOverride(svc.name, 'chaos_level', e.target.value)}
                      className="mt-1 px-2 py-1 border border-border rounded bg-card text-foreground"
                    />
                  </label>
                  <label className="flex flex-col">
                    <span className="text-muted-foreground">Failure rate (0.0–1.0)</span>
                    <input
                      type="number"
                      step="0.1"
                      min="0"
                      max="1"
                      value={overrides[svc.name]?.failure_rate ?? ''}
                      onChange={(e) => updateOverride(svc.name, 'failure_rate', e.target.value)}
                      className="mt-1 px-2 py-1 border border-border rounded bg-card text-foreground"
                    />
                  </label>
                  <label className="flex flex-col">
                    <span className="text-muted-foreground">Latency (ms)</span>
                    <input
                      type="number"
                      step="10"
                      min="0"
                      value={overrides[svc.name]?.latency_ms ?? ''}
                      onChange={(e) => updateOverride(svc.name, 'latency_ms', e.target.value)}
                      className="mt-1 px-2 py-1 border border-border rounded bg-card text-foreground"
                    />
                  </label>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="flex justify-end gap-2">
        <button
          type="button"
          onClick={onCancel}
          className="px-4 py-2 border border-border rounded-lg hover:bg-accent hover:text-accent-foreground transition-colors"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={handleSubmit}
          disabled={pending || !scenarioName.trim()}
          className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {pending ? 'Activating…' : 'Activate'}
        </button>
      </div>
    </div>
  );
};
