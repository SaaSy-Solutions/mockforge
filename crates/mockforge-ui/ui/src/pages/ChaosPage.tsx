import React, { useState, useEffect, useCallback } from 'react';
import { Play, Pause, Square, RefreshCw, Zap, Settings, Wifi, AlertCircle, Gauge, RotateCcw, Loader2 } from 'lucide-react';
import {
  PageHeader,
  ModernCard,
  Alert,
  Section,
  ModernBadge
} from '../components/ui/DesignSystem';
import { Slider } from '../components/ui/slider';
import { Button } from '../components/ui/button';
import { Switch } from '../components/ui/switch';
import { Spinner } from '../components/ui/LoadingStates';
import { StatusBadge } from '../components/ui/StatusBadge';
import {
  useChaosConfig,
  useChaosStatus,
  useUpdateChaosLatency,
  useUpdateChaosFaults,
  useUpdateChaosTraffic,
  useResetChaos,
} from '../hooks/useApi';
import type { ChaosLatencyConfig, ChaosFaultInjectionConfig, ChaosTrafficShapingConfig, CorruptionType } from '../types';
import { toast } from 'sonner';

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

  // Chaos API hooks
  const { data: chaosConfig, isLoading: configLoading, isError: configError } = useChaosConfig();
  const { data: chaosStatus, isLoading: statusLoading } = useChaosStatus();
  const updateLatency = useUpdateChaosLatency();
  const updateFaults = useUpdateChaosFaults();
  const updateTraffic = useUpdateChaosTraffic();
  const resetChaos = useResetChaos();

  // Local state for controls with debouncing
  const [latencyConfig, setLatencyConfig] = useState<ChaosLatencyConfig>({
    enabled: false,
    fixed_delay_ms: null,
    random_delay_range_ms: null,
    jitter_percent: 0,
    probability: 1.0,
  });

  const [faultConfig, setFaultConfig] = useState<ChaosFaultInjectionConfig>({
    enabled: false,
    http_errors: [500, 502, 503, 504],
    http_error_probability: 0.1,
    connection_errors: false,
    connection_error_probability: 0.05,
    timeout_errors: false,
    timeout_ms: 5000,
    timeout_probability: 0.05,
    partial_responses: false,
    partial_response_probability: 0.05,
    payload_corruption: false,
    payload_corruption_probability: 0.05,
    corruption_type: 'none',
  });

  const [trafficConfig, setTrafficConfig] = useState<ChaosTrafficShapingConfig>({
    enabled: false,
    bandwidth_limit_bps: 0,
    packet_loss_percent: 0,
    max_connections: 0,
    connection_timeout_ms: 30000,
  });

  // Debounce timers
  const debounceTimers = React.useRef<Record<string, NodeJS.Timeout>>({});

  // Loading states for each mutation (combine local state with React Query state)
  const updatingLatency = updateLatency.isPending || updateLatency.isError;
  const updatingFaults = updateFaults.isPending || updateFaults.isError;
  const updatingTraffic = updateTraffic.isPending || updateTraffic.isError;

  // Initialize from API config
  useEffect(() => {
    if (chaosConfig) {
      if (chaosConfig.latency) {
        setLatencyConfig(chaosConfig.latency);
      }
      if (chaosConfig.fault_injection) {
        setFaultConfig(chaosConfig.fault_injection);
      }
      if (chaosConfig.traffic_shaping) {
        setTrafficConfig(chaosConfig.traffic_shaping);
      }
    }
  }, [chaosConfig]);

  // Debounced update function with loading state
  const debouncedUpdate = useCallback((
    key: string,
    updateFn: (config: any) => Promise<any>,
    config: any,
    delay: number = 300
  ) => {
    if (debounceTimers.current[key]) {
      clearTimeout(debounceTimers.current[key]);
    }

    debounceTimers.current[key] = setTimeout(() => {
      updateFn(config)
        .then(() => {
          toast.success('Configuration updated', {
            description: 'Chaos settings have been applied successfully',
            duration: 2000,
          });
        })
        .catch((err) => {
          toast.error('Failed to update configuration', {
            description: err.message || 'An error occurred while updating chaos settings',
            duration: 4000,
          });
        });
    }, delay);
  }, []);

  // Cleanup timers on unmount
  useEffect(() => {
    return () => {
      Object.values(debounceTimers.current).forEach(timer => clearTimeout(timer));
    };
  }, []);

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

  if (loading || configLoading) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Chaos Engineering"
          subtitle="Control and monitor chaos scenarios"
        />
        <div className="flex items-center justify-center py-12">
          <div className="text-center space-y-4">
            <Spinner size="lg" />
            <p className="text-gray-600 dark:text-gray-400">Loading chaos configuration...</p>
          </div>
        </div>
      </div>
    );
  }

  if (configError) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Chaos Engineering"
          subtitle="Control and monitor chaos scenarios"
        />
        <Alert
          type="error"
          title="Failed to Load Configuration"
          message="Unable to fetch chaos configuration. Please refresh the page."
        />
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

      {/* Quick Controls */}
      <Section
        title="Quick Controls"
        subtitle="Adjust chaos parameters on the fly with real-time sliders"
      >
        <div className="space-y-6">
          {/* Status Summary */}
          {(latencyConfig.enabled || faultConfig.enabled || trafficConfig.enabled) && (
            <ModernCard className="bg-gradient-to-r from-orange-50 to-red-50 dark:from-orange-900/20 dark:to-red-900/20 border-orange-200 dark:border-orange-800 shadow-lg">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="relative">
                    <Zap className="h-6 w-6 text-orange-600 dark:text-orange-400 animate-pulse" />
                    <div className="absolute inset-0 h-6 w-6 bg-orange-400 rounded-full opacity-75 animate-ping" />
                  </div>
                  <div className="flex-1">
                    <h4 className="font-semibold text-gray-900 dark:text-gray-100 text-lg">
                      Chaos Engineering Active
                    </h4>
                    <div className="flex items-center gap-4 mt-1">
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        {[
                          latencyConfig.enabled && `Latency (${latencyConfig.fixed_delay_ms || 0}ms)`,
                          faultConfig.enabled && `Faults (${(faultConfig.http_error_probability * 100).toFixed(0)}%)`,
                          trafficConfig.enabled && `Traffic (${trafficConfig.packet_loss_percent.toFixed(1)}% loss)`
                        ].filter(Boolean).join(' • ')}
                      </p>
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <StatusBadge status="running" />
                  {(updatingLatency || updatingFaults || updatingTraffic) && (
                    <Spinner size="sm" className="text-orange-600" />
                  )}
                </div>
              </div>
            </ModernCard>
          )}

          {/* Reset All Button */}
          <div className="flex justify-end">
            <Button
              variant="outline"
              onClick={() => {
                resetChaos.mutate();
                toast.success('Chaos configuration reset to defaults');
              }}
              disabled={resetChaos.isPending}
            >
              {resetChaos.isPending ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <RotateCcw className="h-4 w-4 mr-2" />
              )}
              Reset All
            </Button>
          </div>

          {/* Latency Controls */}
          <ModernCard className={latencyConfig.enabled ? 'ring-2 ring-blue-500/50 dark:ring-blue-400/50' : ''}>
            <div className="flex items-center gap-3 mb-6">
              <div className="relative">
                <Zap className={`h-5 w-5 ${latencyConfig.enabled ? 'text-blue-500' : 'text-gray-400'}`} />
                {latencyConfig.enabled && (
                  <div className="absolute inset-0 h-5 w-5 bg-blue-400 rounded-full opacity-75 animate-ping" />
                )}
              </div>
              <h4 className="font-semibold text-gray-900 dark:text-gray-100">Latency Injection</h4>
              <div className="ml-auto flex items-center gap-2">
                {updatingLatency && (
                  <Spinner size="sm" className="text-blue-500" />
                )}
                {latencyConfig.enabled && (
                  <StatusBadge status="running" size="sm" />
                )}
                <Switch
                  checked={latencyConfig.enabled}
                  onCheckedChange={(checked) => {
                    const newConfig = { ...latencyConfig, enabled: checked };
                    setLatencyConfig(newConfig);
                    debouncedUpdate('latency', updateLatency.mutateAsync, newConfig);
                  }}
                  disabled={updatingLatency || configLoading}
                />
              </div>
            </div>
            {latencyConfig.enabled && (
              <div className="space-y-4">
                <Slider
                  label="Fixed Delay"
                  min={0}
                  max={5000}
                  step={50}
                  value={latencyConfig.fixed_delay_ms || 0}
                  onChange={(value) => {
                    const newConfig = { ...latencyConfig, fixed_delay_ms: value };
                    setLatencyConfig(newConfig);
                    debouncedUpdate('latency', updateLatency.mutateAsync, newConfig);
                  }}
                  unit="ms"
                  description="Add a consistent delay to all requests"
                  disabled={updatingLatency || configLoading || !latencyConfig.enabled}
                />
                <Slider
                  label="Jitter"
                  min={0}
                  max={100}
                  step={1}
                  value={latencyConfig.jitter_percent}
                  onChange={(value) => {
                    const newConfig = { ...latencyConfig, jitter_percent: value };
                    setLatencyConfig(newConfig);
                    debouncedUpdate('latency', updateLatency.mutateAsync, newConfig);
                  }}
                  unit="%"
                  description="Random variance applied to delays"
                  disabled={updatingLatency || configLoading || !latencyConfig.enabled}
                />
                <Slider
                  label="Probability"
                  min={0}
                  max={100}
                  step={1}
                  value={latencyConfig.probability * 100}
                  onChange={(value) => {
                    const newConfig = { ...latencyConfig, probability: value / 100 };
                    setLatencyConfig(newConfig);
                    debouncedUpdate('latency', updateLatency.mutateAsync, newConfig);
                  }}
                  unit="%"
                  description="Percentage of requests that will have latency applied"
                  disabled={updatingLatency || configLoading || !latencyConfig.enabled}
                />
              </div>
            )}
          </ModernCard>

          {/* Fault Injection Controls */}
          <ModernCard className={faultConfig.enabled ? 'ring-2 ring-red-500/50 dark:ring-red-400/50' : ''}>
            <div className="flex items-center gap-3 mb-6">
              <div className="relative">
                <AlertCircle className={`h-5 w-5 ${faultConfig.enabled ? 'text-red-500' : 'text-gray-400'}`} />
                {faultConfig.enabled && (
                  <div className="absolute inset-0 h-5 w-5 bg-red-400 rounded-full opacity-75 animate-ping" />
                )}
              </div>
              <h4 className="font-semibold text-gray-900 dark:text-gray-100">Fault Injection</h4>
              <div className="ml-auto flex items-center gap-2">
                {updatingFaults && (
                  <Spinner size="sm" className="text-red-500" />
                )}
                {faultConfig.enabled && (
                  <StatusBadge status="warning" size="sm" />
                )}
                <Switch
                  checked={faultConfig.enabled}
                  onCheckedChange={(checked) => {
                    const newConfig = { ...faultConfig, enabled: checked };
                    setFaultConfig(newConfig);
                    debouncedUpdate('faults', updateFaults.mutateAsync, newConfig);
                  }}
                  disabled={updatingFaults || configLoading}
                />
              </div>
            </div>
            {faultConfig.enabled && (
              <div className="space-y-4">
                <Slider
                  label="HTTP Error Rate"
                  min={0}
                  max={100}
                  step={1}
                  value={faultConfig.http_error_probability * 100}
                  onChange={(value) => {
                    const newConfig = { ...faultConfig, http_error_probability: value / 100 };
                    setFaultConfig(newConfig);
                    debouncedUpdate('faults', updateFaults.mutateAsync, newConfig);
                  }}
                  unit="%"
                  description="Probability of injecting HTTP errors"
                  disabled={updatingFaults || configLoading || !faultConfig.enabled}
                />
                <div className="flex items-center gap-4">
                    <Switch
                    checked={faultConfig.connection_errors}
                    onCheckedChange={(checked) => {
                      const newConfig = { ...faultConfig, connection_errors: checked };
                      setFaultConfig(newConfig);
                      debouncedUpdate('faults', updateFaults.mutateAsync, newConfig);
                    }}
                    disabled={updatingFaults || configLoading || !faultConfig.enabled}
                  />
                  <div className="flex-1">
                    <Slider
                      label="Connection Error Rate"
                      min={0}
                      max={100}
                      step={1}
                      value={faultConfig.connection_error_probability * 100}
                      onChange={(value) => {
                        const newConfig = { ...faultConfig, connection_error_probability: value / 100 };
                        setFaultConfig(newConfig);
                        debouncedUpdate('faults', updateFaults.mutateAsync, newConfig);
                      }}
                      unit="%"
                      description="Probability of connection disconnects"
                      disabled={!faultConfig.connection_errors || updatingFaults || configLoading || !faultConfig.enabled}
                    />
                  </div>
                </div>
                <div className="flex items-center gap-4">
                  <Switch
                    checked={faultConfig.timeout_errors}
                    onCheckedChange={(checked) => {
                      const newConfig = { ...faultConfig, timeout_errors: checked };
                      setFaultConfig(newConfig);
                      debouncedUpdate('faults', updateFaults.mutateAsync, newConfig);
                    }}
                    disabled={updatingFaults || configLoading || !faultConfig.enabled}
                  />
                  <div className="flex-1">
                    <Slider
                      label="Timeout Error Rate"
                      min={0}
                      max={100}
                      step={1}
                      value={faultConfig.timeout_probability * 100}
                      onChange={(value) => {
                        const newConfig = { ...faultConfig, timeout_probability: value / 100 };
                        setFaultConfig(newConfig);
                        debouncedUpdate('faults', updateFaults.mutateAsync, newConfig);
                      }}
                      unit="%"
                      description="Probability of timeout errors"
                      disabled={!faultConfig.timeout_errors || updatingFaults || configLoading || !faultConfig.enabled}
                    />
                  </div>
                </div>
                <div className="flex items-center gap-4">
                  <Switch
                    checked={faultConfig.payload_corruption}
                    onCheckedChange={(checked) => {
                      const newConfig = { ...faultConfig, payload_corruption: checked };
                      setFaultConfig(newConfig);
                      debouncedUpdate('faults', updateFaults.mutateAsync, newConfig);
                    }}
                    disabled={updatingFaults || configLoading || !faultConfig.enabled}
                  />
                  <div className="flex-1 space-y-2">
                    <Slider
                      label="Payload Corruption Rate"
                      min={0}
                      max={100}
                      step={1}
                      value={faultConfig.payload_corruption_probability * 100}
                      onChange={(value) => {
                        const newConfig = { ...faultConfig, payload_corruption_probability: value / 100 };
                        setFaultConfig(newConfig);
                        debouncedUpdate('faults', updateFaults.mutateAsync, newConfig);
                      }}
                      unit="%"
                      description="Probability of corrupting response payloads"
                      disabled={!faultConfig.payload_corruption || updatingFaults || configLoading || !faultConfig.enabled}
                    />
                    <div>
                      <label className="block text-sm text-gray-500 dark:text-gray-400 mb-1">
                        Corruption Type
                      </label>
                      <select
                        value={faultConfig.corruption_type}
                        onChange={(e) => {
                          const newConfig = { ...faultConfig, corruption_type: e.target.value as CorruptionType };
                          setFaultConfig(newConfig);
                          debouncedUpdate('faults', updateFaults.mutateAsync, newConfig);
                        }}
                        disabled={!faultConfig.payload_corruption || updatingFaults || configLoading || !faultConfig.enabled}
                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        <option value="none">None</option>
                        <option value="random_bytes">Random Bytes</option>
                        <option value="truncate">Truncate</option>
                        <option value="bit_flip">Bit Flip</option>
                      </select>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </ModernCard>

          {/* Traffic Shaping Controls */}
          <ModernCard className={trafficConfig.enabled ? 'ring-2 ring-green-500/50 dark:ring-green-400/50' : ''}>
            <div className="flex items-center gap-3 mb-6">
              <div className="relative">
                <Wifi className={`h-5 w-5 ${trafficConfig.enabled ? 'text-green-500' : 'text-gray-400'}`} />
                {trafficConfig.enabled && (
                  <div className="absolute inset-0 h-5 w-5 bg-green-400 rounded-full opacity-75 animate-ping" />
                )}
              </div>
              <h4 className="font-semibold text-gray-900 dark:text-gray-100">Traffic Shaping</h4>
              <div className="ml-auto flex items-center gap-2">
                {updatingTraffic && (
                  <Spinner size="sm" className="text-green-500" />
                )}
                {trafficConfig.enabled && (
                  <StatusBadge status="running" size="sm" />
                )}
                <Switch
                  checked={trafficConfig.enabled}
                  onCheckedChange={(checked) => {
                    const newConfig = { ...trafficConfig, enabled: checked };
                    setTrafficConfig(newConfig);
                    debouncedUpdate('traffic', updateTraffic.mutateAsync, newConfig);
                  }}
                  disabled={updatingTraffic || configLoading}
                />
              </div>
            </div>
            {trafficConfig.enabled && (
              <div className="space-y-4">
                <Slider
                  label="Packet Loss"
                  min={0}
                  max={100}
                  step={0.1}
                  value={trafficConfig.packet_loss_percent}
                  onChange={(value) => {
                    const newConfig = { ...trafficConfig, packet_loss_percent: value };
                    setTrafficConfig(newConfig);
                    debouncedUpdate('traffic', updateTraffic.mutateAsync, newConfig);
                  }}
                  unit="%"
                  description="Percentage of packets to drop (simulates network issues)"
                  disabled={updatingTraffic || configLoading || !trafficConfig.enabled}
                />
                <Slider
                  label="Bandwidth Limit"
                  min={0}
                  max={10000000}
                  step={100000}
                  value={trafficConfig.bandwidth_limit_bps}
                  onChange={(value) => {
                    const newConfig = { ...trafficConfig, bandwidth_limit_bps: value };
                    setTrafficConfig(newConfig);
                    debouncedUpdate('traffic', updateTraffic.mutateAsync, newConfig);
                  }}
                  unit="bps"
                  description="Maximum bandwidth in bytes per second (0 = unlimited)"
                  disabled={updatingTraffic || configLoading || !trafficConfig.enabled}
                />
                <Slider
                  label="Max Connections"
                  min={0}
                  max={1000}
                  step={10}
                  value={trafficConfig.max_connections}
                  onChange={(value) => {
                    const newConfig = { ...trafficConfig, max_connections: value };
                    setTrafficConfig(newConfig);
                    debouncedUpdate('traffic', updateTraffic.mutateAsync, newConfig);
                  }}
                  description="Maximum concurrent connections (0 = unlimited)"
                  disabled={updatingTraffic || configLoading || !trafficConfig.enabled}
                />
              </div>
            )}
          </ModernCard>
        </div>
      </Section>
    </div>
  );
}
