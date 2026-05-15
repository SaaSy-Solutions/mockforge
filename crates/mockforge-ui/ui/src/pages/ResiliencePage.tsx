import React, { useEffect, useState } from 'react';
import { useWebSocket } from '../hooks/useWebSocket';
import { isCloudMode } from '../utils/cloudMode';
import { cloudResilienceApi, type RuntimeState } from '../services/api/cloudResilience';

interface DeploymentSummary {
  id: string;
  name: string;
  status: string;
}

interface CircuitBreakerState {
  endpoint: string;
  state: string;
  stats: {
    total_requests: number;
    successful_requests: number;
    failed_requests: number;
    rejected_requests: number;
    consecutive_failures: number;
    consecutive_successes: number;
    success_rate: number;
    failure_rate: number;
  };
}

interface BulkheadState {
  service: string;
  stats: {
    active_requests: number;
    queued_requests: number;
    total_requests: number;
    rejected_requests: number;
    timeout_requests: number;
    utilization_percent: number;
  };
}

interface DashboardSummary {
  circuit_breakers: {
    total: number;
    open: number;
    half_open: number;
    closed: number;
  };
  bulkheads: {
    total: number;
    active_requests: number;
    queued_requests: number;
  };
}

export const ResiliencePage: React.FC = () => {
  const [circuitBreakers, setCircuitBreakers] = useState<CircuitBreakerState[]>([]);
  const [bulkheads, setBulkheads] = useState<BulkheadState[]>([]);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [selectedTab, setSelectedTab] = useState<'circuit-breakers' | 'bulkheads'>('circuit-breakers');
  const [autoRefresh, setAutoRefresh] = useState(true);
  // `runtime_state: 'pending'` from the cloud scaffold (#468) means the
  // runtime hasn't wired up middleware yet; we want to swap the auto-refresh
  // banner for an honest "pending" notice rather than showing zeros that
  // look like live data.
  const [runtimeState, setRuntimeState] = useState<RuntimeState | null>(null);

  const cloudMode = isCloudMode();
  // In cloud mode the page scopes to a single hosted-mock deployment because
  // resilience state lives in that deployment's running mockforge process.
  // We pick the first active deployment by default; if the org has more
  // than one, the dropdown lets the user switch.
  const [deployments, setDeployments] = useState<DeploymentSummary[]>([]);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<string | null>(null);

  useEffect(() => {
    if (!cloudMode) return;
    let cancelled = false;
    (async () => {
      try {
        const token = localStorage.getItem('auth_token');
        const response = await fetch('/api/v1/hosted-mocks', {
          headers: token ? { Authorization: `Bearer ${token}` } : {},
        });
        if (!response.ok) return;
        const list = (await response.json()) as DeploymentSummary[];
        if (cancelled) return;
        const items = Array.isArray(list) ? list : [];
        setDeployments(items);
        // Auto-select the first active deployment so the page works without
        // an extra click in the common one-deployment-per-org case.
        if (!selectedDeploymentId) {
          const active = items.find((d) => d.status === 'active') ?? items[0] ?? null;
          if (active) setSelectedDeploymentId(active.id);
        }
      } catch (err) {
        console.error('Failed to load deployments:', err);
      }
    })();
    return () => {
      cancelled = true;
    };
    // selectedDeploymentId intentionally omitted: only run on mode change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cloudMode]);

  const fetchCircuitBreakers = async () => {
    try {
      if (cloudMode) {
        if (!selectedDeploymentId) {
          setCircuitBreakers([]);
          return;
        }
        const env = await cloudResilienceApi.listCircuitBreakers(selectedDeploymentId);
        setCircuitBreakers(env.data);
        setRuntimeState(env.runtime_state);
        return;
      }
      const response = await fetch('/api/resilience/circuit-breakers');
      const data = await response.json();
      setCircuitBreakers(Array.isArray(data) ? data : []);
    } catch (error) {
      console.error('Failed to fetch circuit breakers:', error);
    }
  };

  const fetchBulkheads = async () => {
    try {
      if (cloudMode) {
        if (!selectedDeploymentId) {
          setBulkheads([]);
          return;
        }
        const env = await cloudResilienceApi.listBulkheads(selectedDeploymentId);
        setBulkheads(env.data);
        setRuntimeState(env.runtime_state);
        return;
      }
      const response = await fetch('/api/resilience/bulkheads');
      const data = await response.json();
      setBulkheads(Array.isArray(data) ? data : []);
    } catch (error) {
      console.error('Failed to fetch bulkheads:', error);
    }
  };

  const fetchSummary = async () => {
    try {
      if (cloudMode) {
        if (!selectedDeploymentId) {
          setSummary(null);
          return;
        }
        const s = await cloudResilienceApi.getSummary(selectedDeploymentId);
        setSummary({
          circuit_breakers: s.circuit_breakers,
          bulkheads: { ...s.bulkheads },
        });
        setRuntimeState(s.runtime_state);
        return;
      }
      const response = await fetch('/api/resilience/dashboard/summary');
      const data = await response.json();
      setSummary(data && typeof data === 'object' && !Array.isArray(data) ? data : null);
    } catch (error) {
      console.error('Failed to fetch summary:', error);
    }
  };

  const resetCircuitBreaker = async (endpoint: string) => {
    try {
      if (cloudMode) {
        if (!selectedDeploymentId) return;
        const result = await cloudResilienceApi.resetCircuitBreaker(
          selectedDeploymentId,
          endpoint,
        );
        if (!result.accepted) {
          console.info('Circuit breaker reset is a no-op:', result.reason);
        }
        fetchCircuitBreakers();
        return;
      }
      await fetch(`/api/resilience/circuit-breakers/${encodeURIComponent(endpoint)}/reset`, {
        method: 'POST',
      });
      fetchCircuitBreakers();
    } catch (error) {
      console.error('Failed to reset circuit breaker:', error);
    }
  };

  const resetBulkhead = async (service: string) => {
    try {
      if (cloudMode) {
        if (!selectedDeploymentId) return;
        const result = await cloudResilienceApi.resetBulkhead(selectedDeploymentId, service);
        if (!result.accepted) {
          console.info('Bulkhead reset is a no-op:', result.reason);
        }
        fetchBulkheads();
        return;
      }
      await fetch(`/api/resilience/bulkheads/${encodeURIComponent(service)}/reset`, {
        method: 'POST',
      });
      fetchBulkheads();
    } catch (error) {
      console.error('Failed to reset bulkhead:', error);
    }
  };

  useEffect(() => {
    fetchCircuitBreakers();
    fetchBulkheads();
    fetchSummary();

    const interval = setInterval(() => {
      if (autoRefresh) {
        fetchCircuitBreakers();
        fetchBulkheads();
        fetchSummary();
      }
    }, 3000);

    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoRefresh, selectedDeploymentId]);

  const getStateColor = (state: string): string => {
    switch (state) {
      case 'Closed':
        return 'bg-success-500';
      case 'Open':
        return 'bg-danger-500';
      case 'HalfOpen':
        return 'bg-warning-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getUtilizationColor = (percent: number): string => {
    if (percent < 50) return 'bg-success-500';
    if (percent < 80) return 'bg-warning-500';
    return 'bg-danger-500';
  };

  return (
    <div className="p-6 space-y-6">
      {/* Deployment unreachable banner. The registry proxies live state from
          the hosted-mock's admin port; when that proxy fails (deployment
          stopped, not yet started, transient network), the page renders zeros
          and we want to surface why instead of letting them look like real
          live data. */}
      {cloudMode && runtimeState === 'unreachable' && selectedDeploymentId && (
        <div className="bg-warning-50 border border-warning-200 dark:bg-warning-900/30 dark:border-warning-800 rounded-lg p-4">
          <p className="font-medium text-warning-700 dark:text-warning-300">
            Deployment not reachable
          </p>
          <p className="text-sm text-warning-700/80 dark:text-warning-300/80 mt-1">
            The registry could not reach this deployment&rsquo;s admin endpoint.
            Resilience counters will populate once the deployment is running
            and reachable on the private network.
          </p>
        </div>
      )}
      {cloudMode && !selectedDeploymentId && (
        <div className="bg-card border border-border rounded-lg p-4 text-sm text-muted-foreground">
          {deployments.length === 0
            ? 'No hosted-mock deployments yet. Create one from the Hosted Mocks page to see resilience state here.'
            : 'Select a deployment to view resilience state.'}
        </div>
      )}
      {cloudMode && deployments.length > 1 && (
        <div className="flex items-center space-x-2">
          <label className="text-sm font-medium text-foreground" htmlFor="resilience-deployment">
            Deployment:
          </label>
          <select
            id="resilience-deployment"
            value={selectedDeploymentId ?? ''}
            onChange={(e) => setSelectedDeploymentId(e.target.value || null)}
            className="rounded border border-border bg-card text-foreground text-sm px-2 py-1"
          >
            {deployments.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name} ({d.status})
              </option>
            ))}
          </select>
        </div>
      )}

      {/* Header */}
      <div className="flex justify-between items-center">
        <h1 className="text-3xl font-bold text-foreground">Resilience Dashboard</h1>
        <div className="flex items-center space-x-4">
          <label className="flex items-center space-x-2">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
              className="form-checkbox h-5 w-5 text-info-600"
            />
            <span className="text-sm text-foreground">Auto-refresh (3s)</span>
          </label>
          <button
            onClick={() => {
              fetchCircuitBreakers();
              fetchBulkheads();
              fetchSummary();
            }}
            className="px-4 py-2 bg-info-500 text-white rounded hover:bg-primary"
          >
            Refresh Now
          </button>
        </div>
      </div>

      {/* Summary Cards */}
      {summary && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="bg-card rounded-lg shadow p-6">
            <h2 className="text-xl font-semibold text-foreground mb-4">Circuit Breakers</h2>
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Total:</span>
                <span className="font-semibold">{summary.circuit_breakers?.total ?? 0}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Closed:</span>
                <span className="font-semibold text-success-600">{summary.circuit_breakers?.closed ?? 0}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Half-Open:</span>
                <span className="font-semibold text-warning-600">{summary.circuit_breakers?.half_open ?? 0}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Open:</span>
                <span className="font-semibold text-danger-600">{summary.circuit_breakers?.open ?? 0}</span>
              </div>
            </div>
          </div>

          <div className="bg-card rounded-lg shadow p-6">
            <h2 className="text-xl font-semibold text-foreground mb-4">Bulkheads</h2>
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Total Services:</span>
                <span className="font-semibold">{summary.bulkheads?.total ?? 0}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Active Requests:</span>
                <span className="font-semibold text-info-600">{summary.bulkheads?.active_requests ?? 0}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Queued Requests:</span>
                <span className="font-semibold text-warning-600">{summary.bulkheads?.queued_requests ?? 0}</span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Tabs */}
      <div className="border-b border-border">
        <nav className="-mb-px flex space-x-8">
          <button
            onClick={() => setSelectedTab('circuit-breakers')}
            className={`${
              selectedTab === 'circuit-breakers'
                ? 'border-info text-info-600'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
            } whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm`}
          >
            Circuit Breakers
          </button>
          <button
            onClick={() => setSelectedTab('bulkheads')}
            className={`${
              selectedTab === 'bulkheads'
                ? 'border-info text-info-600'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
            } whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm`}
          >
            Bulkheads
          </button>
        </nav>
      </div>

      {/* Circuit Breakers Tab */}
      {selectedTab === 'circuit-breakers' && (
        <div className="space-y-4">
          {circuitBreakers.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              No circuit breakers configured
            </div>
          ) : (
            circuitBreakers.map((cb) => (
              <div key={cb.endpoint} className="bg-card rounded-lg shadow p-6">
                <div className="flex justify-between items-start mb-4">
                  <div className="flex items-center space-x-3">
                    <div className={`w-3 h-3 rounded-full ${getStateColor(cb.state)}`} />
                    <h3 className="text-lg font-semibold text-foreground">{cb.endpoint}</h3>
                    <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                      cb.state === 'Closed' ? 'bg-success-100 text-success-700' :
                      cb.state === 'Open' ? 'bg-danger-100 text-danger-700' :
                      'bg-warning-100 text-warning-700'
                    }`}>
                      {cb.state}
                    </span>
                  </div>
                  <button
                    onClick={() => resetCircuitBreaker(cb.endpoint)}
                    className="px-3 py-1 bg-gray-200 text-foreground rounded hover:bg-gray-300 text-sm"
                  >
                    Reset
                  </button>
                </div>

                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  <div>
                    <div className="text-sm text-muted-foreground">Total Requests</div>
                    <div className="text-2xl font-semibold">{cb.stats?.total_requests ?? 0}</div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Success Rate</div>
                    <div className="text-2xl font-semibold text-success-600">
                      {(cb.stats?.success_rate ?? 0).toFixed(1)}%
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Failure Rate</div>
                    <div className="text-2xl font-semibold text-danger-600">
                      {(cb.stats?.failure_rate ?? 0).toFixed(1)}%
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Rejected</div>
                    <div className="text-2xl font-semibold text-orange-600">
                      {cb.stats?.rejected_requests ?? 0}
                    </div>
                  </div>
                </div>

                <div className="mt-4 grid grid-cols-2 gap-4 text-sm">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Successful:</span>
                    <span className="font-medium">{cb.stats?.successful_requests ?? 0}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Failed:</span>
                    <span className="font-medium">{cb.stats?.failed_requests ?? 0}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Consecutive Failures:</span>
                    <span className="font-medium">{cb.stats?.consecutive_failures ?? 0}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Consecutive Successes:</span>
                    <span className="font-medium">{cb.stats?.consecutive_successes ?? 0}</span>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      )}

      {/* Bulkheads Tab */}
      {selectedTab === 'bulkheads' && (
        <div className="space-y-4">
          {bulkheads.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              No bulkheads configured
            </div>
          ) : (
            bulkheads.map((bh) => (
              <div key={bh.service} className="bg-card rounded-lg shadow p-6">
                <div className="flex justify-between items-start mb-4">
                  <h3 className="text-lg font-semibold text-foreground">{bh.service}</h3>
                  <button
                    onClick={() => resetBulkhead(bh.service)}
                    className="px-3 py-1 bg-gray-200 text-foreground rounded hover:bg-gray-300 text-sm"
                  >
                    Reset Stats
                  </button>
                </div>

                {/* Utilization Bar */}
                <div className="mb-4">
                  <div className="flex justify-between text-sm mb-1">
                    <span className="text-muted-foreground">Utilization</span>
                    <span className="font-medium">{(bh.stats?.utilization_percent ?? 0).toFixed(1)}%</span>
                  </div>
                  <div className="w-full bg-gray-200 rounded-full h-3">
                    <div
                      className={`h-3 rounded-full transition-all ${getUtilizationColor(bh.stats?.utilization_percent ?? 0)}`}
                      style={{ width: `${Math.min(bh.stats?.utilization_percent ?? 0, 100)}%` }}
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
                  <div>
                    <div className="text-sm text-muted-foreground">Active</div>
                    <div className="text-2xl font-semibold text-info-600">
                      {bh.stats?.active_requests ?? 0}
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Queued</div>
                    <div className="text-2xl font-semibold text-warning-600">
                      {bh.stats?.queued_requests ?? 0}
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Total</div>
                    <div className="text-2xl font-semibold">{bh.stats?.total_requests ?? 0}</div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Rejected</div>
                    <div className="text-2xl font-semibold text-danger-600">
                      {bh.stats?.rejected_requests ?? 0}
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Timeouts</div>
                    <div className="text-2xl font-semibold text-orange-600">
                      {bh.stats?.timeout_requests ?? 0}
                    </div>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};
