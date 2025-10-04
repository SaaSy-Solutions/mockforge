import React, { useState, useEffect } from 'react';
import { Settings, Save, RefreshCw, Shield, Zap, Server, Database, Wifi, WifiOff } from 'lucide-react';
import { useConfig, useValidation, useServerInfo, useUpdateLatency, useUpdateFaults, useUpdateProxy, useUpdateValidation, useRestartServers, useRestartStatus } from '../hooks/useApi';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import { toast } from 'sonner';
import {
  PageHeader,
  ModernCard,
  ModernBadge,
  Section
} from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose
} from '../components/ui/Dialog';
import { EnvironmentManager } from '../components/workspace/EnvironmentManager';
import { AutocompleteInput } from '../components/ui/AutocompleteInput';

function extractPort(address?: string): string {
  if (!address) return '';
  const parts = address.split(':');
  return parts[parts.length - 1] || '';
}

// Validation functions
function isValidUrl(url: string): boolean {
  if (!url) return true; // Empty URL is valid (optional field)
  try {
    const urlObj = new URL(url);
    return urlObj.protocol === 'http:' || urlObj.protocol === 'https:';
  } catch {
    return false;
  }
}

function isValidPort(port: number): boolean {
  return port >= 1 && port <= 65535;
}

export function ConfigPage() {
  const [activeSection, setActiveSection] = useState<'general' | 'latency' | 'faults' | 'traffic-shaping' | 'proxy' | 'validation' | 'environment'>('general');
  const { activeWorkspace } = useWorkspaceStore();
  const workspaceId = activeWorkspace?.id || 'default-workspace';
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const [showRestartDialog, setShowRestartDialog] = useState(false);

  const { data: config, isLoading: configLoading } = useConfig();
  const { data: validation, isLoading: validationLoading } = useValidation();
  const { data: serverInfo, isLoading: serverInfoLoading } = useServerInfo();

  // Mutations
  const updateLatency = useUpdateLatency();
  const updateFaults = useUpdateFaults();
  const updateProxy = useUpdateProxy();
  const updateValidation = useUpdateValidation();
  const restartServers = useRestartServers();
  const { data: restartStatus } = useRestartStatus();

  const [formData, setFormData] = useState({
    general: {
      http_port: 3000,
      ws_port: 3001,
      grpc_port: 50051,
      admin_port: 9080
    },
    restartInProgress: false,
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

  // Save port configuration to localStorage for persistence across restarts
  const savePortConfig = (ports: typeof formData.general) => {
    localStorage.setItem('mockforge_pending_port_config', JSON.stringify(ports));
  };

  // Load pending port configuration on mount
  useEffect(() => {
    const pendingConfig = localStorage.getItem('mockforge_pending_port_config');
    if (pendingConfig) {
      try {
        const ports = JSON.parse(pendingConfig);
        setFormData(prev => ({
          ...prev,
          general: { ...prev.general, ...ports }
        }));
      } catch (error) {
        console.error('Failed to parse pending port config:', error);
        localStorage.removeItem('mockforge_pending_port_config');
      }
    }
  }, []);

  // Monitor restart status
  useEffect(() => {
    if (restartStatus && formData.restartInProgress) {
      if (!restartStatus.restarting) {
        setFormData(prev => ({ ...prev, restartInProgress: false }));
        toast.success('Server restarted successfully! Port configuration applied.');
        localStorage.removeItem('mockforge_pending_port_config');
      }
    }
  }, [restartStatus, formData.restartInProgress]);

  // Initialize form data from API when data loads
  useEffect(() => {
    if (config?.latency) {
      setFormData(prev => ({
        ...prev,
        latency: {
          base_ms: config.latency.base_ms,
          jitter_ms: config.latency.jitter_ms
        }
      }));
    }
    if (config?.faults) {
      setFormData(prev => ({
        ...prev,
        faults: {
          enabled: config.faults.enabled,
          failure_rate: config.faults.failure_rate,
          status_codes: config.faults.status_codes
        }
      }));
    }
    if (config?.proxy) {
      setFormData(prev => ({
        ...prev,
        proxy: {
          enabled: config.proxy.enabled,
          upstream_url: config.proxy.upstream_url || '',
          timeout_seconds: config.proxy.timeout_seconds
        }
      }));
    }
  }, [config]);

  useEffect(() => {
    if (serverInfo) {
      setFormData(prev => ({
        ...prev,
        general: {
          http_port: parseInt(extractPort(serverInfo.http_server)) || 3000,
          ws_port: parseInt(extractPort(serverInfo.ws_server)) || 3001,
          grpc_port: parseInt(extractPort(serverInfo.grpc_server)) || 50051,
          admin_port: serverInfo.admin_port || 9080
        }
      }));
    }
  }, [serverInfo]);

  useEffect(() => {
    if (validation) {
      setFormData(prev => ({
        ...prev,
        validation: {
          mode: validation.mode as 'enforce' | 'warn' | 'off',
          aggregate_errors: validation.aggregate_errors,
          validate_responses: validation.validate_responses,
          overrides: validation.overrides
        }
      }));
    }
  }, [validation]);

  // Detect changes by comparing current form data to server data
  useEffect(() => {
    let hasChanges = false;

    // Check general settings
    if (serverInfo) {
      const currentHttpPort = parseInt(extractPort(serverInfo.http_server)) || 3000;
      const currentWsPort = parseInt(extractPort(serverInfo.ws_server)) || 3001;
      const currentGrpcPort = parseInt(extractPort(serverInfo.grpc_server)) || 50051;
      const currentAdminPort = serverInfo.admin_port || 9080;

      if (formData.general.http_port !== currentHttpPort ||
          formData.general.ws_port !== currentWsPort ||
          formData.general.grpc_port !== currentGrpcPort ||
          formData.general.admin_port !== currentAdminPort) {
        hasChanges = true;
      }
    }

    // Check latency settings
    if (config?.latency) {
      if (formData.latency.base_ms !== config.latency.base_ms ||
          formData.latency.jitter_ms !== config.latency.jitter_ms) {
        hasChanges = true;
      }
    }

    // Check fault settings
    if (config?.faults) {
      if (formData.faults.enabled !== config.faults.enabled ||
          formData.faults.failure_rate !== config.faults.failure_rate ||
          JSON.stringify(formData.faults.status_codes) !== JSON.stringify(config.faults.status_codes)) {
        hasChanges = true;
      }
    }

    // Check proxy settings
    if (config?.proxy) {
      if (formData.proxy.enabled !== config.proxy.enabled ||
          formData.proxy.upstream_url !== (config.proxy.upstream_url || '') ||
          formData.proxy.timeout_seconds !== config.proxy.timeout_seconds) {
        hasChanges = true;
      }
    }

    // Check validation settings
    if (validation) {
      if (formData.validation.mode !== validation.mode ||
          formData.validation.aggregate_errors !== validation.aggregate_errors ||
          formData.validation.validate_responses !== validation.validate_responses) {
        hasChanges = true;
      }
    }

    setHasUnsavedChanges(hasChanges);
  }, [formData, config, validation, serverInfo]);

  // Warn before leaving page with unsaved changes
  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (hasUnsavedChanges) {
        e.preventDefault();
        e.returnValue = '';
      }
    };

    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => window.removeEventListener('beforeunload', handleBeforeUnload);
  }, [hasUnsavedChanges]);

  const handleSave = async (section: string) => {
    // Validate before saving
    if (section === 'proxy' && formData.proxy.enabled) {
      if (!formData.proxy.upstream_url) {
        toast.error('Upstream URL is required when proxy is enabled');
        return;
      }
      if (!isValidUrl(formData.proxy.upstream_url)) {
        toast.error('Invalid upstream URL. Must be a valid HTTP or HTTPS URL');
        return;
      }
      if (!isValidPort(formData.proxy.timeout_seconds)) {
        toast.error('Invalid timeout. Must be between 1 and 300 seconds');
        return;
      }
    }

    if (section === 'general') {
      if (!isValidPort(formData.general.http_port)) {
        toast.error('Invalid HTTP port. Must be between 1 and 65535');
        return;
      }
      if (!isValidPort(formData.general.ws_port)) {
        toast.error('Invalid WebSocket port. Must be between 1 and 65535');
        return;
      }
      if (!isValidPort(formData.general.grpc_port)) {
        toast.error('Invalid gRPC port. Must be between 1 and 65535');
        return;
      }
      if (!isValidPort(formData.general.admin_port)) {
        toast.error('Invalid Admin port. Must be between 1 and 65535');
        return;
      }
    }

    try {
      switch (section) {
        case 'latency':
          await updateLatency.mutateAsync({
            name: 'default',
            base_ms: formData.latency.base_ms,
            jitter_ms: formData.latency.jitter_ms,
            tag_overrides: {}
          });
          toast.success('Latency configuration saved successfully');
          break;

        case 'faults':
          await updateFaults.mutateAsync({
            enabled: formData.faults.enabled,
            failure_rate: formData.faults.failure_rate,
            status_codes: formData.faults.status_codes,
            active_failures: 0
          });
          toast.success('Fault injection configuration saved successfully');
          break;

        case 'proxy':
          await updateProxy.mutateAsync({
            enabled: formData.proxy.enabled,
            upstream_url: formData.proxy.upstream_url,
            timeout_seconds: formData.proxy.timeout_seconds,
            requests_proxied: 0
          });
          toast.success('Proxy configuration saved successfully');
          break;

        case 'validation':
          await updateValidation.mutateAsync({
            mode: formData.validation.mode,
            aggregate_errors: formData.validation.aggregate_errors,
            validate_responses: formData.validation.validate_responses,
            overrides: formData.validation.overrides
          });
          toast.success('Validation settings saved successfully');
          break;

        case 'general': {
          // Save port configuration to localStorage for persistence
          savePortConfig(formData.general);

          // Show confirmation dialog
          setShowRestartDialog(true);
          break;
        }

        case 'traffic-shaping':
          // Traffic shaping configuration
          try {
            const response = await fetch('/__mockforge/config/traffic-shaping', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                config_type: 'traffic-shaping',
                data: formData.trafficShaping
              })
            });

            if (!response.ok) {
              throw new Error(`HTTP error! status: ${response.status}`);
            }

            toast.success('Traffic shaping configuration saved successfully');
          } catch (error) {
            console.error('Error saving traffic shaping:', error);
            toast.error('Failed to save traffic shaping configuration');
          }
          break;

        default:
          toast.error(`Unknown section: ${section}`);
      }
    } catch (error) {
      console.error(`Error saving ${section} configuration:`, error);
      toast.error(`Failed to save ${section} configuration`);
    }
  };

  const handleConfirmRestart = async () => {
    setShowRestartDialog(false);
    try {
      setFormData(prev => ({ ...prev, restartInProgress: true }));
      toast.info('Saving configuration and restarting server...');

      await restartServers.mutateAsync('Port configuration updated');

      // The restart status monitoring will handle success feedback
    } catch (error) {
      setFormData(prev => ({ ...prev, restartInProgress: false }));
      toast.error('Failed to restart server. Please restart manually.');
      console.error('Server restart failed:', error);
    }
  };

  const handleCancelRestart = () => {
    setShowRestartDialog(false);
    toast.info('Configuration saved locally. Restart the server manually to apply changes.');
  };

  const handleReset = (section: string) => {
    switch (section) {
      case 'general':
        if (serverInfo) {
          setFormData(prev => ({
            ...prev,
            general: {
              http_port: parseInt(extractPort(serverInfo.http_server)) || 3000,
              ws_port: parseInt(extractPort(serverInfo.ws_server)) || 3001,
              grpc_port: parseInt(extractPort(serverInfo.grpc_server)) || 50051,
              admin_port: serverInfo.admin_port || 9080
            }
          }));
          toast.info('General settings reset to server values');
        }
        break;

      case 'latency':
        if (config?.latency) {
          setFormData(prev => ({
            ...prev,
            latency: {
              base_ms: config.latency.base_ms,
              jitter_ms: config.latency.jitter_ms
            }
          }));
          toast.info('Latency configuration reset to server values');
        }
        break;

      case 'faults':
        if (config?.faults) {
          setFormData(prev => ({
            ...prev,
            faults: {
              enabled: config.faults.enabled,
              failure_rate: config.faults.failure_rate,
              status_codes: config.faults.status_codes
            }
          }));
          toast.info('Fault injection configuration reset to server values');
        }
        break;

      case 'proxy':
        if (config?.proxy) {
          setFormData(prev => ({
            ...prev,
            proxy: {
              enabled: config.proxy.enabled,
              upstream_url: config.proxy.upstream_url || '',
              timeout_seconds: config.proxy.timeout_seconds
            }
          }));
          toast.info('Proxy configuration reset to server values');
        }
        break;

      case 'validation':
        if (validation) {
          setFormData(prev => ({
            ...prev,
            validation: {
              mode: validation.mode as 'enforce' | 'warn' | 'off',
              aggregate_errors: validation.aggregate_errors,
              validate_responses: validation.validate_responses,
              overrides: validation.overrides
            }
          }));
          toast.info('Validation settings reset to server values');
        }
        break;

      case 'traffic-shaping':
        setFormData(prev => ({
          ...prev,
          trafficShaping: {
            enabled: false,
            bandwidth: {
              enabled: false,
              max_bytes_per_sec: 1048576,
              burst_capacity_bytes: 10485760
            },
            burstLoss: {
              enabled: false,
              burst_probability: 0.1,
              burst_duration_ms: 5000,
              loss_rate_during_burst: 0.5,
              recovery_time_ms: 30000
            }
          }
        }));
        toast.info('Traffic shaping configuration reset to defaults');
        break;

      default:
        toast.error(`Unknown section: ${section}`);
    }
  };

  const handleResetAll = () => {
    handleReset('general');
    handleReset('latency');
    handleReset('faults');
    handleReset('traffic-shaping');
    handleReset('proxy');
    handleReset('validation');
    toast.success('All settings reset to server values');
  };

  const handleSaveAll = async () => {
    const sections = ['general', 'latency', 'faults', 'traffic-shaping', 'proxy', 'validation'];
    let successCount = 0;
    let errorCount = 0;

    for (const section of sections) {
      try {
        await handleSave(section);
        successCount++;
      } catch (_error) {
        errorCount++;
      }
    }

    if (errorCount === 0) {
      toast.success('All settings saved successfully');
    } else {
      toast.warning(`Saved ${successCount} sections, ${errorCount} failed`);
    }
  };

  if (configLoading || validationLoading || serverInfoLoading) {
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
        subtitle={
          hasUnsavedChanges
            ? "⚠️ You have unsaved changes"
            : "Manage MockForge settings and preferences"
        }
        action={
          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              className="flex items-center gap-2"
              onClick={handleResetAll}
            >
              <RefreshCw className="h-4 w-4" />
              Reset All
            </Button>
            <Button
              variant="default"
              size="sm"
              className="flex items-center gap-2"
              onClick={handleSaveAll}
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
                        <Input
                          type="number"
                          min="1"
                          max="65535"
                          value={formData.general.http_port}
                          onChange={(e) => setFormData(prev => ({
                            ...prev,
                            general: { ...prev.general, http_port: parseInt(e.target.value) || 3000 }
                          }))}
                        />
                      </div>
                      <div>
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                          WebSocket Port
                        </label>
                        <Input
                          type="number"
                          min="1"
                          max="65535"
                          value={formData.general.ws_port}
                          onChange={(e) => setFormData(prev => ({
                            ...prev,
                            general: { ...prev.general, ws_port: parseInt(e.target.value) || 3001 }
                          }))}
                        />
                      </div>
                      <div>
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                          gRPC Port
                        </label>
                        <Input
                          type="number"
                          min="1"
                          max="65535"
                          value={formData.general.grpc_port}
                          onChange={(e) => setFormData(prev => ({
                            ...prev,
                            general: { ...prev.general, grpc_port: parseInt(e.target.value) || 50051 }
                          }))}
                        />
                      </div>
                      <div>
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                          Admin Port
                        </label>
                        <Input
                          type="number"
                          min="1"
                          max="65535"
                          value={formData.general.admin_port}
                          onChange={(e) => setFormData(prev => ({
                            ...prev,
                            general: { ...prev.general, admin_port: parseInt(e.target.value) || 9080 }
                          }))}
                        />
                      </div>
                    </div>
                  </div>

                  {formData.restartInProgress && (
                    <div className="flex items-center gap-2 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg mb-4">
                      <RefreshCw className="w-4 h-4 animate-spin text-blue-600" />
                      <span className="text-sm text-blue-700 dark:text-blue-300">
                        Server restart in progress... Configuration will be applied shortly.
                      </span>
                    </div>
                  )}

                  <div className="flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700">
                    <Button variant="outline" onClick={() => handleReset('general')} disabled={formData.restartInProgress}>
                      Reset
                    </Button>
                    <Button onClick={() => handleSave('general')} disabled={formData.restartInProgress}>
                      {formData.restartInProgress ? (
                        <>
                          <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                          Restarting...
                        </>
                      ) : (
                        'Save & Restart Server'
                      )}
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
                          className={
                            formData.proxy.upstream_url && !isValidUrl(formData.proxy.upstream_url)
                              ? 'border-red-500 dark:border-red-500'
                              : ''
                          }
                        />
                        {formData.proxy.upstream_url && !isValidUrl(formData.proxy.upstream_url) && (
                          <p className="text-xs text-red-600 dark:text-red-400 mt-1">
                            Must be a valid HTTP or HTTPS URL
                          </p>
                        )}
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
                onEnvironmentSelect={(_envId) => {
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

      {/* Restart Confirmation Dialog */}
      <Dialog open={showRestartDialog} onOpenChange={setShowRestartDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Restart Server Required</DialogTitle>
            <DialogClose onClick={handleCancelRestart} />
          </DialogHeader>
          <DialogDescription>
            Port configuration changes require a server restart to take effect.
          </DialogDescription>
          <div className="py-4">
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="font-medium">HTTP Port:</span>
                <span>{formData.general.http_port}</span>
              </div>
              <div className="flex justify-between">
                <span className="font-medium">WebSocket Port:</span>
                <span>{formData.general.ws_port}</span>
              </div>
              <div className="flex justify-between">
                <span className="font-medium">gRPC Port:</span>
                <span>{formData.general.grpc_port}</span>
              </div>
              <div className="flex justify-between">
                <span className="font-medium">Admin Port:</span>
                <span>{formData.general.admin_port}</span>
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={handleCancelRestart}>
              Cancel
            </Button>
            <Button onClick={handleConfirmRestart}>
              Restart Server
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
