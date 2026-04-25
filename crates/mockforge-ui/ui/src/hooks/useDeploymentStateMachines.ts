/**
 * Pulls state-machine definitions and instances from a hosted-mock
 * deployment via the cloud-side proxy. Mirrors useDeploymentCaptures —
 * polling, no SSE, since the data is small and changes infrequently
 * relative to request traffic.
 */

import { useEffect, useState } from 'react';
import { logger } from '@/utils/logger';

export interface DeploymentStateMachineSummary {
  resource_type: string;
  state_count: number;
  transition_count: number;
  sub_scenario_count: number;
  has_visual_layout: boolean;
}

export interface DeploymentStateMachineInstance {
  resource_id: string;
  resource_type: string;
  current_state: string;
  history_count: number;
  state_data: Record<string, unknown>;
}

export interface UseDeploymentStateMachinesOptions {
  enabled?: boolean;
  intervalMs?: number;
}

export interface UseDeploymentStateMachinesReturn {
  machines: DeploymentStateMachineSummary[];
  instances: DeploymentStateMachineInstance[];
  loading: boolean;
  error: string | null;
  refetch: () => void;
}

export function useDeploymentStateMachines(
  deploymentId: string | undefined,
  options: UseDeploymentStateMachinesOptions = {},
): UseDeploymentStateMachinesReturn {
  const { enabled = true, intervalMs = 10000 } = options;

  const [machines, setMachines] = useState<DeploymentStateMachineSummary[]>([]);
  const [instances, setInstances] = useState<DeploymentStateMachineInstance[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refetchTick, setRefetchTick] = useState(0);

  useEffect(() => {
    if (!enabled || !deploymentId) {
      return;
    }

    let cancelled = false;

    const poll = async () => {
      const token = localStorage.getItem('auth_token');
      if (!token) return;

      const base = `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/state-machines`;
      setLoading(true);
      try {
        const [definitionsResp, instancesResp] = await Promise.all([
          fetch(base, { headers: { Authorization: `Bearer ${token}` } }),
          fetch(`${base}/instances`, { headers: { Authorization: `Bearer ${token}` } }),
        ]);

        if (cancelled) return;

        if (definitionsResp.status === 404) {
          // Deployment doesn't expose state-machine API yet — surface as
          // empty rather than error.
          setMachines([]);
          setInstances([]);
          setError(null);
          return;
        }

        if (!definitionsResp.ok) {
          throw new Error(`HTTP ${definitionsResp.status}`);
        }

        const defsData = await definitionsResp.json();
        const machinesList: DeploymentStateMachineSummary[] = Array.isArray(defsData?.state_machines)
          ? defsData.state_machines
          : [];
        setMachines(machinesList);

        if (instancesResp.ok) {
          const instData = await instancesResp.json();
          const instancesList: DeploymentStateMachineInstance[] = Array.isArray(instData?.instances)
            ? instData.instances
            : [];
          setInstances(instancesList);
        } else {
          setInstances([]);
        }

        setError(null);
      } catch (err) {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : 'Failed to fetch state machines';
        setError(msg);
        logger.warn('Deployment state machines poll failed', err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    void poll();
    const handle = setInterval(poll, intervalMs);
    return () => {
      cancelled = true;
      clearInterval(handle);
    };
  }, [enabled, deploymentId, intervalMs, refetchTick]);

  return {
    machines,
    instances,
    loading,
    error,
    refetch: () => setRefetchTick((t) => t + 1),
  };
}
