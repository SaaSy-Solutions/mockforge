import React, { useState } from 'react';
import { Settings, Save, RefreshCw, Shield, Zap, Server, Database, Wifi, WifiOff } from 'lucide-react';
import { useConfig, useValidation } from '../hooks/useApi';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import {
  PageHeader,
  ModernCard,
  ModernBadge,
  Section
} from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { EnvironmentManager } from '../components/workspace/EnvironmentManager';
import { AutocompleteInput } from '../components/ui/AutocompleteInput';

export function ConfigPage() {
  const [activeSection, setActiveSection] = useState<'general' | 'latency' | 'faults' | 'traffic-shaping' | 'proxy' | 'validation' | 'environment'>('general');
  const { activeWorkspace } = useWorkspaceStore();
  const workspaceId = activeWorkspace?.id || 'default-workspace';

  const { isLoading: configLoading } = useConfig();
  const { isLoading: validationLoading } = useValidation();

  const [formData, setFormData] = useState({
    latency: { base_ms: 0, jitter_ms: 0 },
    faults: { enabled: false, failure_rate: 0, status_codes: [] as number[] },
    trafficShaping: {
      enabled: false,
      bandwidth: {
        enabled: false,
        max_bytes_per_sec: 1048576, // 1 MB/s
        burst_capacity_bytes: 10485760 // 10 MB
      },
      burstLoss: {
        enabled: false,
        burst_probability: 0.1,
        burst_duration_ms: 5000,
        loss_rate_during_burst: 0.5,
        recovery_time_ms: 30000
      }
    },
    proxy: { enabled: false, upstream_url: '', timeout_seconds: 30 },
    validation: {
      mode: 'enforce' as 'enforce' | 'warn' | 'off',
      aggregate_errors: true,
      validate_responses: true,
      overrides: {} as Record<string, string>
    },
    templateTest: ''
  });

  const handleSave = (section: string) => {
    console.log(`Saving ${section} configuration:`, formData[section as keyof typeof formData]);
    // Here you would make API calls to save the configuration
  };

  const handleReset = (section: string) => {
    console.log(`Resetting ${section} configuration`);
    // Here you would reset the form data
  };

  if (configLoading || validationLoading) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Configuration"
          subtitle="Manage MockForge settings and preferences"
        />
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <span className="ml-3 text-lg text-gray-600 dark:text-gray-400">Loading configuration...</span>
        </div>
      </div>
    );
  }

  const sections = [
    { id: 'general', label: 'General', icon: Settings, description: 'Basic MockForge settings' },
    { id: 'latency', label: 'Latency', icon: Zap, description: 'Response delay and timing' },
    { id: 'faults', label: 'Fault Injection', icon: Shield, description: 'Error simulation and failure modes' },
    { id: 'traffic-shaping', label: 'Traffic Shaping', icon: Wifi, description: 'Bandwidth control and network simulation' },
    { id: 'proxy', label: 'Proxy', icon: Server, description: 'Upstream proxy configuration' },
    { id: 'validation', label: 'Validation', icon: Database, description: 'Request/response validation' },
    { id: 'environment', label: 'Environment', icon: Settings, description: 'Environment variables' },
  ];

  return (
    <div className="space-y-8">
      <PageHeader
        title="Configuration"
        subtitle="Manage MockForge settings and preferences"
        action={
          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              className="flex items-center gap-2"
            >
              <RefreshCw className="h-4 w-4" />
              Reset All
            </Button>
            <Button
              variant="default"
              size="sm"
              className="flex items-center gap-2"
            >
              <Save className="h-4 w-4" />
              Save All Changes
            </Button>
          </div>
        }
      />

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-8">
        {/* Navigation Sidebar */}
        <div className="lg:col-span-1">
          <ModernCard>
            <nav className="space-y-2">
              {sections.map((section) => {
                const Icon = section.icon;
                return (
                  <button
                    key={section.id}
                    onClick={() => setActiveSection(section.id as typeof activeSection)}
                    className={`w-full flex items-center gap-3 px-3 py-3 rounded-lg text-left transition-colors ${
                      activeSection === section.id
                        ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300'
                        : 'hover:bg-gray-50 dark:hover:bg-gray-800/50 text-gray-700 dark:text-gray-300'
                    }`}
                  >
                    <Icon className="h-5 w-5" />
                    <div>
                      <div className="font-medium">{section.label}</div>
                      <div className="text-xs opacity-75">{section.description}</div>
                    </div>
                  </button>
                );
              })}
            </nav>
          </ModernCard>
        </div>

        {/* Main Content */}
        <div className="lg:col-span-3">
          {activeSection === 'general' && (
            <Section title="General Settings" subtitle="Basic MockForge configuration">
              <ModernCard>
                <div className="space-y-6">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                      Server Configuration
                    </label>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      <div>
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                          HTTP Port
                        </label>
                        <Input type="number" defaultValue="3000" />
                      </div>
                      <div>
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                          WebSocket Port
                        </label>
                        <Input type="number" defaultValue="3001" />
                      </div>
                      <div>
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                          gRPC Port
                        </label>
                        <Input type="number" defaultValue="50051" />
                      </div>
                      <div>
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                          Admin Port
                        </label>
                        <Input type="number" defaultValue="8080" />
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700">
                    <Button variant="outline" onClick={() => handleReset('general')}>
                      Reset
                    </Button>
                    <Button onClick={() => handleSave('general')}>
                      Save Changes
                    </Button>
                  </div>
                </div>
              </ModernCard>
            </Section>
          )}

          {activeSection === 'latency' && (
            <Section title="Latency Configuration" subtitle="Control response timing and delays">
              <ModernCard>
                <div className="space-y-6">
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                        Base Latency (ms)
                      </label>
                      <Input
                        type="number"
                        placeholder="0"
                        value={formData.latency.base_ms}
                        onChange={(e) => setFormData(prev => ({
                          ...prev,
                          latency: { ...prev.latency, base_ms: parseInt(e.target.value) || 0 }
                        }))}
                      />
                      <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        Minimum response time for all requests
                      </p>
                    </div>

                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                        Jitter (ms)
                      </label>
                      <Input
                        type="number"
                        placeholder="0"
                        value={formData.latency.jitter_ms}
                        onChange={(e) => setFormData(prev => ({
                          ...prev,
                          latency: { ...prev.latency, jitter_ms: parseInt(e.target.value) || 0 }
                        }))}
                      />
                      <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        Random delay variation (± jitter)
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700">
                    <Button variant="outline" onClick={() => handleReset('latency')}>
                      Reset
                    </Button>
                    <Button onClick={() => handleSave('latency')}>
                      Save Changes
                    </Button>
                  </div>
                </div>
              </ModernCard>
            </Section>
          )}

          {activeSection === 'faults' && (
            <Section title="Fault Injection" subtitle="Configure error simulation and failure scenarios">
              <ModernCard>
                <div className="space-y-6">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100">
                        Enable Fault Injection
                      </h3>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        Simulate network failures and server errors
                      </p>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input
                        type="checkbox"
                        className="sr-only peer"
                        checked={formData.faults.enabled}
                        onChange={(e) => setFormData(prev => ({
                          ...prev,
                          faults: { ...prev.faults, enabled: e.target.checked }
                        }))}
                      />
                      <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                    </label>
                  </div>

                  {formData.faults.enabled && (
                    <>
                      <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                          Failure Rate (%)
                        </label>
                        <Input
                          type="number"
                          min="0"
                          max="100"
                          placeholder="5"
                          value={formData.faults.failure_rate}
                          onChange={(e) => setFormData(prev => ({
                            ...prev,
                            faults: { ...prev.faults, failure_rate: parseInt(e.target.value) || 0 }
                          }))}
                        />
                      </div>

                      <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                          Error Status Codes
                        </label>
                      <div className="flex flex-wrap gap-2">
                        {[500, 502, 503, 504, 400, 401, 403, 404].map(code => (
                          <button
                            key={code}
                            onClick={() => {
                              setFormData(prev => ({
                                ...prev,
                                faults: {
                                  ...prev.faults,
                                  status_codes: prev.faults.status_codes.includes(code)
                                    ? prev.faults.status_codes.filter(c => c !== code)
                                    : [...prev.faults.status_codes, code]
                                }
                              }));
                            }}
                            className="cursor-pointer"
                          >
                            <ModernBadge
                              variant={formData.faults.status_codes.includes(code) ? 'error' : 'outline'}
                            >
                              {code}
                            </ModernBadge>
                          </button>
                        ))}
                      </div>
                      </div>
                    </>
                  )}

                  <div className="flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700">
                    <Button variant="outline" onClick={() => handleReset('faults')}>
                      Reset
                    </Button>
                    <Button onClick={() => handleSave('faults')}>
                      Save Changes
                    </Button>
                  </div>
                </div>
              </ModernCard>
            </Section>
          )}

          {activeSection === 'traffic-shaping' && (
            <Section title="Traffic Shaping" subtitle="Control bandwidth and simulate network conditions">
              <ModernCard>
                <div className="space-y-8">
                  {/* Overall Traffic Shaping Toggle */}
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100">
                        Enable Traffic Shaping
                      </h3>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        Simulate real network conditions with bandwidth control and connectivity issues
                      </p>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input
                        type="checkbox"
                        className="sr-only peer"
                        checked={formData.trafficShaping.enabled}
                        onChange={(e) => setFormData(prev => ({
                          ...prev,
                          trafficShaping: { ...prev.trafficShaping, enabled: e.target.checked }
                        }))}
                      />
                      <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                    </label>
                  </div>

                  {formData.trafficShaping.enabled && (
                    <>
                      {/* Bandwidth Control Section */}
                      <div className="border-t border-gray-200 dark:border-gray-700 pt-6">
                        <div className="flex items-center gap-3 mb-4">
                          <Wifi className="h-5 w-5 text-blue-600" />
                          <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100">
                            Bandwidth Control
                          </h3>
                        </div>

                        <div className="flex items-center justify-between mb-4">
                          <div>
                            <h4 className="text-sm font-medium text-gray-900 dark:text-gray-100">
                              Enable Bandwidth Throttling
                            </h4>
                            <p className="text-sm text-gray-600 dark:text-gray-400">
                              Limit data transfer rates using token bucket algorithm
                            </p>
                          </div>
                          <label className="relative inline-flex items-center cursor-pointer">
                            <input
                              type="checkbox"
                              className="sr-only peer"
                              checked={formData.trafficShaping.bandwidth.enabled}
                              onChange={(e) => setFormData(prev => ({
                                ...prev,
                                trafficShaping: {
                                  ...prev.trafficShaping,
                                  bandwidth: { ...prev.trafficShaping.bandwidth, enabled: e.target.checked }
                                }
                              }))}
                            />
                            <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                          </label>
                        </div>

                        {formData.trafficShaping.bandwidth.enabled && (
                          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                            <div>
                              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                Max Bandwidth (bytes/sec)
                              </label>
                              <Input
                                type="number"
                                min="1"
                                placeholder="1048576"
                                value={formData.trafficShaping.bandwidth.max_bytes_per_sec}
                                onChange={(e) => setFormData(prev => ({
                                  ...prev,
                                  trafficShaping: {
                                    ...prev.trafficShaping,
                                    bandwidth: {
                                      ...prev.trafficShaping.bandwidth,
                                      max_bytes_per_sec: parseInt(e.target.value) || 1048576
                                    }
                                  }
                                }))}
                              />
                              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                Maximum data transfer rate (1 MB/s = 1,048,576 bytes)
                              </p>
                            </div>

                            <div>
                              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                Burst Capacity (bytes)
                              </label>
                              <Input
                                type="number"
                                min="1"
                                placeholder="10485760"
                                value={formData.trafficShaping.bandwidth.burst_capacity_bytes}
                                onChange={(e) => setFormData(prev => ({
                                  ...prev,
                                  trafficShaping: {
                                    ...prev.trafficShaping,
                                    bandwidth: {
                                      ...prev.trafficShaping.bandwidth,
                                      burst_capacity_bytes: parseInt(e.target.value) || 10485760
                                    }
                                  }
                                }))}
                              />
                              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                Token bucket capacity for burst traffic (10 MB = 10,485,760 bytes)
                              </p>
                            </div>
                          </div>
                        )}
                      </div>

                      {/* Burst Loss Section */}
                      <div className="border-t border-gray-200 dark:border-gray-700 pt-6">
                        <div className="flex items-center gap-3 mb-4">
                          <WifiOff className="h-5 w-5 text-orange-600" />
                          <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100">
                            Burst Loss Simulation
                          </h3>
                        </div>

                        <div className="flex items-center justify-between mb-4">
                          <div>
                            <h4 className="text-sm font-medium text-gray-900 dark:text-gray-100">
                              Enable Burst Loss
                            </h4>
                            <p className="text-sm text-gray-600 dark:text-gray-400">
                              Simulate intermittent connectivity issues and packet loss
                            </p>
                          </div>
                          <label className="relative inline-flex items-center cursor-pointer">
                            <input
                              type="checkbox"
                              className="sr-only peer"
                              checked={formData.trafficShaping.burstLoss.enabled}
                              onChange={(e) => setFormData(prev => ({
                                ...prev,
                                trafficShaping: {
                                  ...prev.trafficShaping,
                                  burstLoss: { ...prev.trafficShaping.burstLoss, enabled: e.target.checked }
                                }
                              }))}
                            />
                            <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                          </label>
                        </div>

                        {formData.trafficShaping.burstLoss.enabled && (
                          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                            <div>
                              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                Burst Probability (%)
                              </label>
                              <Input
                                type="number"
                                min="0"
                                max="100"
                                step="0.1"
                                placeholder="10"
                                value={formData.trafficShaping.burstLoss.burst_probability * 100}
                                onChange={(e) => setFormData(prev => ({
                                  ...prev,
                                  trafficShaping: {
                                    ...prev.trafficShaping,
                                    burstLoss: {
                                      ...prev.trafficShaping.burstLoss,
                                      burst_probability: parseFloat(e.target.value) / 100 || 0.1
                                    }
                                  }
                                }))}
                              />
                              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                Probability of entering a loss burst (0-100%)
                              </p>
                            </div>

                            <div>
                              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                Burst Duration (ms)
                              </label>
                              <Input
                                type="number"
                                min="100"
                                placeholder="5000"
                                value={formData.trafficShaping.burstLoss.burst_duration_ms}
                                onChange={(e) => setFormData(prev => ({
                                  ...prev,
                                  trafficShaping: {
                                    ...prev.trafficShaping,
                                    burstLoss: {
                                      ...prev.trafficShaping.burstLoss,
                                      burst_duration_ms: parseInt(e.target.value) || 5000
                                    }
                                  }
                                }))}
                              />
                              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                Duration of loss bursts in milliseconds
                              </p>
                            </div>

                            <div>
                              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                Loss Rate During Burst (%)
                              </label>
                              <Input
                                type="number"
                                min="0"
                                max="100"
                                step="0.1"
                                placeholder="50"
                                value={formData.trafficShaping.burstLoss.loss_rate_during_burst * 100}
                                onChange={(e) => setFormData(prev => ({
                                  ...prev,
                                  trafficShaping: {
                                    ...prev.trafficShaping,
                                    burstLoss: {
                                      ...prev.trafficShaping.burstLoss,
                                      loss_rate_during_burst: parseFloat(e.target.value) / 100 || 0.5
                                    }
                                  }
                                }))}
                              />
                              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                Packet loss rate during burst periods (0-100%)
                              </p>
                            </div>

                            <div>
                              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                Recovery Time (ms)
                              </label>
                              <Input
                                type="number"
                                min="1000"
                                placeholder="30000"
                                value={formData.trafficShaping.burstLoss.recovery_time_ms}
                                onChange={(e) => setFormData(prev => ({
                                  ...prev,
                                  trafficShaping: {
                                    ...prev.trafficShaping,
                                    burstLoss: {
                                      ...prev.trafficShaping.burstLoss,
                                      recovery_time_ms: parseInt(e.target.value) || 30000
                                    }
                                  }
                                }))}
                              />
                              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                Recovery period between bursts in milliseconds
                              </p>
                            </div>
                          </div>
                        )}
                      </div>
                    </>
                  )}

                  <div className="flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700">
                    <Button variant="outline" onClick={() => handleReset('traffic-shaping')}>
                      Reset
                    </Button>
                    <Button onClick={() => handleSave('traffic-shaping')}>
                      Save Changes
                    </Button>
                  </div>
                </div>
              </ModernCard>
            </Section>
          )}

          {activeSection === 'proxy' && (
            <Section title="Proxy Configuration" subtitle="Configure upstream proxy settings">
              <ModernCard>
                <div className="space-y-6">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100">
                        Enable Proxy Mode
                      </h3>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        Forward requests to upstream services
                      </p>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input
                        type="checkbox"
                        className="sr-only peer"
                        checked={formData.proxy.enabled}
                        onChange={(e) => setFormData(prev => ({
                          ...prev,
                          proxy: { ...prev.proxy, enabled: e.target.checked }
                        }))}
                      />
                      <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                    </label>
                  </div>

                  {formData.proxy.enabled && (
                    <>
                      <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                          Upstream URL
                        </label>
                        <Input
                          type="url"
                          placeholder="https://api.example.com"
                          value={formData.proxy.upstream_url}
                          onChange={(e) => setFormData(prev => ({
                            ...prev,
                            proxy: { ...prev.proxy, upstream_url: e.target.value }
                          }))}
                        />
                      </div>

                      <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                          Timeout (seconds)
                        </label>
                        <Input
                          type="number"
                          min="1"
                          max="300"
                          placeholder="30"
                          value={formData.proxy.timeout_seconds}
                          onChange={(e) => setFormData(prev => ({
                            ...prev,
                            proxy: { ...prev.proxy, timeout_seconds: parseInt(e.target.value) || 30 }
                          }))}
                        />
                      </div>
                    </>
                  )}

                  <div className="flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700">
                    <Button variant="outline" onClick={() => handleReset('proxy')}>
                      Reset
                    </Button>
                    <Button onClick={() => handleSave('proxy')}>
                      Save Changes
                    </Button>
                  </div>
                </div>
              </ModernCard>
            </Section>
          )}

          {activeSection === 'validation' && (
            <Section title="Validation Settings" subtitle="Configure request and response validation">
              <ModernCard>
                <div className="space-y-6">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                      Validation Mode
                    </label>
                    <select
                      value={formData.validation.mode}
                      onChange={(e) => setFormData(prev => ({
                        ...prev,
                        validation: { ...prev.validation, mode: e.target.value as 'enforce' | 'warn' | 'off' }
                      }))}
                      className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
                    >
                      <option value="enforce">Enforce (Strict)</option>
                      <option value="warn">Warn Only</option>
                      <option value="off">Disabled</option>
                    </select>
                  </div>

                  <div className="space-y-4">
                    <div className="flex items-center justify-between">
                      <div>
                        <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100">
                          Aggregate Errors
                        </h3>
                        <p className="text-sm text-gray-600 dark:text-gray-400">
                          Collect all validation errors before responding
                        </p>
                      </div>
                      <input
                        type="checkbox"
                        checked={formData.validation.aggregate_errors}
                        onChange={(e) => setFormData(prev => ({
                          ...prev,
                          validation: { ...prev.validation, aggregate_errors: e.target.checked }
                        }))}
                        className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
                      />
                    </div>

                    <div className="flex items-center justify-between">
                      <div>
                        <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100">
                          Validate Responses
                        </h3>
                        <p className="text-sm text-gray-600 dark:text-gray-400">
                          Check response format and content
                        </p>
                      </div>
                      <input
                        type="checkbox"
                        checked={formData.validation.validate_responses}
                        onChange={(e) => setFormData(prev => ({
                          ...prev,
                          validation: { ...prev.validation, validate_responses: e.target.checked }
                        }))}
                        className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
                      />
                    </div>
                  </div>

                  <div className="flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700">
                    <Button variant="outline" onClick={() => handleReset('validation')}>
                      Reset
                    </Button>
                    <Button onClick={() => handleSave('validation')}>
                      Save Changes
                    </Button>
                  </div>
                </div>
              </ModernCard>
            </Section>
          )}

          {activeSection === 'environment' && (
            <Section title="Environments & Variables" subtitle="Manage environments and their variables">
              <EnvironmentManager
                workspaceId={workspaceId}
                onEnvironmentSelect={(envId) => {
                  console.log('Environment selected:', envId);
                  // Could update URL or notify other components
                }}
              />

              {/* Template Testing Section */}
              <div className="mt-8">
                <ModernCard>
                  <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">
                    Template Testing
                  </h3>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
                    Test variable substitution in templates. Type {'{{'} to see available variables.
                  </p>

                  <div className="space-y-4">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                        Template Input (with autocomplete)
                      </label>
                      <AutocompleteInput
                        value={formData.templateTest || ''}
                        onChange={(value) => setFormData(prev => ({ ...prev, templateTest: value }))}
                        placeholder="Type {{ to see available variables..."
                        workspaceId={workspaceId}
                        context="template_test"
                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
                      />
                    </div>

                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                        Expected Output
                      </label>
                      <div className="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg font-mono text-sm text-gray-600 dark:text-gray-400">
                        {formData.templateTest || 'Template output will appear here...'}
                      </div>
                    </div>

                    <div className="text-xs text-gray-500 dark:text-gray-400">
                      💡 Tip: Use Ctrl+Space anywhere in a text input to manually trigger autocomplete
                    </div>
                  </div>
                </ModernCard>
              </div>
            </Section>
          )}
        </div>
      </div>
    </div>
  );
}
