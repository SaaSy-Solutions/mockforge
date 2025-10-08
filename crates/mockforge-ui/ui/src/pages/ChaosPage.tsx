import React, { useState, useEffect } from 'react';
import { Play, Pause, Square, RefreshCw, Zap, Settings } from 'lucide-react';
import {
  PageHeader,
  ModernCard,
  Alert,
  Section,
  ModernBadge
} from '../components/ui/DesignSystem';

interface ChaosScenario {
  name: string;
  description: string;
  enabled: boolean;
  config: {
    latency?: {
      enabled: boolean;
      fixed_delay_ms?: number;
      probability: number;
    };
    fault_injection?: {
      enabled: boolean;
      http_errors: number[];
      http_error_probability: number;
    };
    rate_limit?: {
      enabled: boolean;
      requests_per_second: number;
    };
    traffic_shaping?: {
      enabled: boolean;
      bandwidth_limit_bps: number;
      packet_loss_percent: number;
    };
  };
}

interface ScenarioStatus {
  active_scenario?: string;
  is_enabled: boolean;
  current_config?: any;
}

export function ChaosPage() {
  const [scenarios, setScenarios] = useState<ChaosScenario[]>([]);
  const [status, setStatus] = useState<ScenarioStatus>({ is_enabled: false });
  const [loading, setLoading] = useState(true);
  const [selectedScenario, setSelectedScenario] = useState<string | null>(null);

  useEffect(() => {
    fetchScenarios();
    fetchStatus();
  }, []);

  const fetchScenarios = async () => {
    try {
      const response = await fetch('/api/chaos/scenarios');
      if (!response.ok) throw new Error('Failed to fetch scenarios');
      const data = await response.json();
      setScenarios(data.scenarios || []);
    } catch (err) {
      console.error('Failed to fetch scenarios:', err);
    } finally {
      setLoading(false);
    }
  };

  const fetchStatus = async () => {
    try {
      const response = await fetch('/api/chaos/status');
      if (!response.ok) throw new Error('Failed to fetch status');
      const data = await response.json();
      setStatus(data);
    } catch (err) {
      console.error('Failed to fetch status:', err);
    }
  };

  const startScenario = async (scenarioName: string) => {
    try {
      const response = await fetch(`/api/chaos/scenarios/${scenarioName}`, {
        method: 'POST',
      });
      if (!response.ok) throw new Error('Failed to start scenario');
      fetchStatus();
    } catch (err) {
      alert(`Failed to start scenario: ${err}`);
    }
  };

  const stopChaos = async () => {
    try {
      const response = await fetch('/api/chaos/disable', {
        method: 'POST',
      });
      if (!response.ok) throw new Error('Failed to stop chaos');
      fetchStatus();
    } catch (err) {
      alert(`Failed to stop chaos: ${err}`);
    }
  };

  const resetChaos = async () => {
    try {
      const response = await fetch('/api/chaos/reset', {
        method: 'POST',
      });
      if (!response.ok) throw new Error('Failed to reset chaos');
      fetchStatus();
    } catch (err) {
      alert(`Failed to reset chaos: ${err}`);
    }
  };

  const predefinedScenarios = [
    {
      name: 'network_degradation',
      description: 'Simulates poor network conditions with latency and packet loss',
      color: 'yellow'
    },
    {
      name: 'service_instability',
      description: 'Introduces random HTTP errors and timeouts',
      color: 'orange'
    },
    {
      name: 'cascading_failure',
      description: 'Simulates cascading failures with high error rates and delays',
      color: 'red'
    },
    {
      name: 'peak_traffic',
      description: 'Enforces aggressive rate limiting to simulate high load',
      color: 'blue'
    },
    {
      name: 'slow_backend',
      description: 'Adds consistent high latency to all requests',
      color: 'purple'
    }
  ];

  if (loading) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Chaos Engineering"
          subtitle="Control and monitor chaos scenarios"
        />
        <Alert type="info" title="Loading" message="Fetching chaos scenarios..." />
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <PageHeader
        title="Chaos Engineering"
        subtitle="Test system resilience with controlled failure injection"
        actions={
          <div className="flex gap-2">
            <button
              onClick={fetchStatus}
              className="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 flex items-center gap-2"
            >
              <RefreshCw className="h-4 w-4" />
              Refresh
            </button>
            {status.is_enabled && (
              <button
                onClick={stopChaos}
                className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 flex items-center gap-2"
              >
                <Square className="h-4 w-4" />
                Stop All Chaos
              </button>
            )}
          </div>
        }
      />

      {/* Status Banner */}
      {status.is_enabled ? (
        <Alert
          type="warning"
          title="Chaos Engineering Active"
          message={`Active scenario: ${status.active_scenario || 'Custom configuration'}`}
          actions={
            <button
              onClick={resetChaos}
              className="px-4 py-2 bg-yellow-600 text-white rounded-lg hover:bg-yellow-700"
            >
              Reset
            </button>
          }
        />
      ) : (
        <Alert
          type="info"
          title="Chaos Engineering Disabled"
          message="Select a scenario below to start testing system resilience"
        />
      )}

      {/* Predefined Scenarios */}
      <Section
        title="Predefined Scenarios"
        subtitle="Ready-to-use chaos scenarios for common failure patterns"
      >
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {predefinedScenarios.map(scenario => (
            <ModernCard key={scenario.name}>
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-center gap-3">
                  <Zap className="h-6 w-6 text-gray-400" />
                  <div>
                    <h3 className="font-semibold text-gray-900 dark:text-gray-100">
                      {scenario.name.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase())}
                    </h3>
                    <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                      {scenario.description}
                    </p>
                  </div>
                </div>
                {status.active_scenario === scenario.name && (
                  <ModernBadge variant="warning">Active</ModernBadge>
                )}
              </div>
              <button
                onClick={() => startScenario(scenario.name)}
                disabled={status.active_scenario === scenario.name}
                className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed flex items-center justify-center gap-2"
              >
                <Play className="h-4 w-4" />
                Start Scenario
              </button>
            </ModernCard>
          ))}
        </div>
      </Section>

      {/* Current Configuration */}
      {status.is_enabled && status.current_config && (
        <Section
          title="Current Configuration"
          subtitle="Active chaos engineering settings"
        >
          <ModernCard>
            <pre className="bg-gray-100 dark:bg-gray-800 p-4 rounded-lg overflow-x-auto">
              <code className="text-sm font-mono">
                {JSON.stringify(status.current_config, null, 2)}
              </code>
            </pre>
          </ModernCard>
        </Section>
      )}

      {/* Latency Control */}
      <Section
        title="Quick Controls"
        subtitle="Adjust chaos parameters on the fly"
      >
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <ModernCard>
            <div className="flex items-center gap-3 mb-4">
              <Settings className="h-5 w-5 text-gray-400" />
              <h4 className="font-semibold text-gray-900 dark:text-gray-100">Latency</h4>
            </div>
            <div className="space-y-3">
              <div>
                <label className="block text-sm text-gray-500 dark:text-gray-400 mb-1">
                  Delay (ms)
                </label>
                <input
                  type="number"
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100"
                  placeholder="500"
                />
              </div>
              <button className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700">
                Apply
              </button>
            </div>
          </ModernCard>

          <ModernCard>
            <div className="flex items-center gap-3 mb-4">
              <Settings className="h-5 w-5 text-gray-400" />
              <h4 className="font-semibold text-gray-900 dark:text-gray-100">Fault Injection</h4>
            </div>
            <div className="space-y-3">
              <div>
                <label className="block text-sm text-gray-500 dark:text-gray-400 mb-1">
                  Error Rate (%)
                </label>
                <input
                  type="number"
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100"
                  placeholder="10"
                  min="0"
                  max="100"
                />
              </div>
              <button className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700">
                Apply
              </button>
            </div>
          </ModernCard>

          <ModernCard>
            <div className="flex items-center gap-3 mb-4">
              <Settings className="h-5 w-5 text-gray-400" />
              <h4 className="font-semibold text-gray-900 dark:text-gray-100">Rate Limiting</h4>
            </div>
            <div className="space-y-3">
              <div>
                <label className="block text-sm text-gray-500 dark:text-gray-400 mb-1">
                  Max RPS
                </label>
                <input
                  type="number"
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100"
                  placeholder="100"
                />
              </div>
              <button className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700">
                Apply
              </button>
            </div>
          </ModernCard>
        </div>
      </Section>
    </div>
  );
}
