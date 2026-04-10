import React, { useState, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/Badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { Switch } from '@/components/ui/switch';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Key,
  Save,
  CheckCircle2,
  XCircle,
  AlertCircle,
  ExternalLink,
  Eye,
  EyeOff,
  Info,
  Trash2,
  Zap,
  Settings2,
  BarChart3,
  ScrollText,
  Loader2,
} from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';

// Types
interface BYOKConfig {
  provider: 'openai' | 'anthropic' | 'together' | 'fireworks' | 'custom';
  api_key: string;
  base_url?: string;
  model?: string;
  enabled: boolean;
}

interface Organization {
  id: string;
  name: string;
  slug: string;
}

interface OrgAiSettings {
  max_ai_calls_per_workspace_per_day: number;
  max_ai_calls_per_workspace_per_month: number;
  feature_flags: {
    ai_studio_enabled: boolean;
    ai_contract_diff_enabled: boolean;
    mockai_enabled: boolean;
    persona_generation_enabled: boolean;
    generative_schema_enabled: boolean;
    voice_interface_enabled: boolean;
  };
}

interface OrgUsage {
  org_id: string;
  total_requests: number;
  total_storage_gb: number;
  total_ai_tokens: number;
  hosted_mocks_count: number;
  plugins_published: number;
  api_tokens_count: number;
}

interface AuditLogEntry {
  id: string;
  event_type: string;
  description: string;
  metadata: Record<string, unknown> | null;
  created_at: string;
  ip_address: string | null;
  user_agent: string | null;
}

interface TestConnectionResult {
  success: boolean;
  message: string;
  details?: string;
}

// API helpers
const API_BASE = '/api/v1';

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('auth_token');
  return {
    Authorization: `Bearer ${token}`,
    'Content-Type': 'application/json',
  };
}

async function fetchOrganizations(): Promise<Organization[]> {
  const response = await fetch(`${API_BASE}/organizations`, {
    headers: authHeaders(),
  });
  if (!response.ok) throw new Error('Failed to fetch organizations');
  return response.json();
}

async function fetchBYOKConfig(reveal: boolean): Promise<BYOKConfig> {
  const response = await fetch(`${API_BASE}/settings/byok?reveal=${reveal}`, {
    headers: authHeaders(),
  });
  if (!response.ok) {
    if (response.status === 404) {
      return { provider: 'openai', api_key: '', enabled: false };
    }
    throw new Error('Failed to fetch BYOK config');
  }
  return response.json();
}

async function saveBYOKConfig(config: BYOKConfig): Promise<void> {
  const response = await fetch(`${API_BASE}/settings/byok`, {
    method: 'PUT',
    headers: authHeaders(),
    body: JSON.stringify(config),
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || 'Failed to save BYOK config');
  }
}

async function deleteBYOKConfig(): Promise<void> {
  const response = await fetch(`${API_BASE}/settings/byok`, {
    method: 'DELETE',
    headers: authHeaders(),
  });
  if (!response.ok) throw new Error('Failed to delete BYOK config');
}

async function testBYOKConnection(config: {
  provider: string;
  api_key: string;
  base_url?: string;
  model?: string;
}): Promise<TestConnectionResult> {
  const response = await fetch(`${API_BASE}/settings/byok/test`, {
    method: 'POST',
    headers: authHeaders(),
    body: JSON.stringify(config),
  });
  if (!response.ok) throw new Error('Failed to test connection');
  return response.json();
}

async function fetchOrgAiSettings(orgId: string): Promise<OrgAiSettings> {
  const response = await fetch(`${API_BASE}/organizations/${orgId}/settings/ai`, {
    headers: authHeaders(),
  });
  if (!response.ok) throw new Error('Failed to fetch AI settings');
  return response.json();
}

async function saveOrgAiSettings(orgId: string, settings: OrgAiSettings): Promise<OrgAiSettings> {
  const response = await fetch(`${API_BASE}/organizations/${orgId}/settings/ai`, {
    method: 'PATCH',
    headers: authHeaders(),
    body: JSON.stringify(settings),
  });
  if (!response.ok) throw new Error('Failed to save AI settings');
  return response.json();
}

async function fetchOrgUsage(orgId: string): Promise<OrgUsage> {
  const response = await fetch(`${API_BASE}/organizations/${orgId}/usage`, {
    headers: authHeaders(),
  });
  if (!response.ok) throw new Error('Failed to fetch usage');
  return response.json();
}

async function fetchAuditLogs(orgId: string): Promise<AuditLogEntry[]> {
  const response = await fetch(
    `${API_BASE}/organizations/${orgId}/audit-logs?event_type=byok_config_updated,byok_config_deleted`,
    { headers: authHeaders() }
  );
  if (!response.ok) throw new Error('Failed to fetch audit logs');
  return response.json();
}

// Provider definitions with model options
const PROVIDERS = [
  {
    value: 'openai' as const,
    label: 'OpenAI',
    description: 'Use OpenAI API (GPT-4, GPT-3.5, etc.)',
    baseUrl: 'https://api.openai.com/v1',
    docsUrl: 'https://platform.openai.com/docs',
    models: [
      { value: 'gpt-4o', label: 'GPT-4o' },
      { value: 'gpt-4o-mini', label: 'GPT-4o Mini' },
      { value: 'gpt-4-turbo', label: 'GPT-4 Turbo' },
      { value: 'gpt-3.5-turbo', label: 'GPT-3.5 Turbo' },
      { value: 'o3-mini', label: 'o3-mini' },
    ],
  },
  {
    value: 'anthropic' as const,
    label: 'Anthropic',
    description: 'Use Anthropic API (Claude)',
    baseUrl: 'https://api.anthropic.com/v1',
    docsUrl: 'https://docs.anthropic.com',
    models: [
      { value: 'claude-sonnet-4-6', label: 'Claude Sonnet 4.6' },
      { value: 'claude-opus-4-6', label: 'Claude Opus 4.6' },
      { value: 'claude-haiku-4-5-20251001', label: 'Claude Haiku 4.5' },
    ],
  },
  {
    value: 'together' as const,
    label: 'Together AI',
    description: 'Use Together AI for open-source models',
    baseUrl: 'https://api.together.xyz/v1',
    docsUrl: 'https://docs.together.ai',
    models: [
      { value: 'meta-llama/Llama-3.3-70B-Instruct-Turbo', label: 'Llama 3.3 70B Instruct' },
      { value: 'meta-llama/Llama-3.1-8B-Instruct-Turbo', label: 'Llama 3.1 8B Instruct' },
      { value: 'mistralai/Mixtral-8x7B-Instruct-v0.1', label: 'Mixtral 8x7B' },
      { value: 'Qwen/Qwen2.5-72B-Instruct-Turbo', label: 'Qwen 2.5 72B' },
    ],
  },
  {
    value: 'fireworks' as const,
    label: 'Fireworks AI',
    description: 'Use Fireworks AI for fast inference',
    baseUrl: 'https://api.fireworks.ai/inference/v1',
    docsUrl: 'https://docs.fireworks.ai',
    models: [
      {
        value: 'accounts/fireworks/models/llama-v3p3-70b-instruct',
        label: 'Llama 3.3 70B Instruct',
      },
      {
        value: 'accounts/fireworks/models/mixtral-8x7b-instruct',
        label: 'Mixtral 8x7B Instruct',
      },
      {
        value: 'accounts/fireworks/models/qwen2p5-72b-instruct',
        label: 'Qwen 2.5 72B Instruct',
      },
    ],
  },
  {
    value: 'custom' as const,
    label: 'Custom',
    description: 'Use a custom OpenAI-compatible API',
    baseUrl: '',
    docsUrl: '',
    models: [],
  },
];

// AI feature flag metadata
const AI_FEATURES = [
  {
    key: 'ai_studio_enabled' as const,
    label: 'AI Studio',
    description: 'AI-powered mock generation and editing',
  },
  {
    key: 'ai_contract_diff_enabled' as const,
    label: 'AI Contract Diff',
    description: 'Intelligent API contract comparison and change detection',
  },
  {
    key: 'mockai_enabled' as const,
    label: 'MockAI',
    description: 'AI-driven dynamic mock responses',
  },
  {
    key: 'persona_generation_enabled' as const,
    label: 'Persona Generation',
    description: 'Generate realistic test data personas',
  },
  {
    key: 'generative_schema_enabled' as const,
    label: 'Generative Schema',
    description: 'Auto-generate schemas from examples',
  },
  {
    key: 'voice_interface_enabled' as const,
    label: 'Voice Interface',
    description: 'Voice-controlled mock configuration',
  },
];

// ────────────────────────────────────────────────────────────────────────────
// Configuration Tab
// ────────────────────────────────────────────────────────────────────────────

function ConfigurationTab() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [showApiKey, setShowApiKey] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [config, setConfig] = useState<BYOKConfig>({
    provider: 'openai',
    api_key: '',
    enabled: false,
  });

  // Fetch config with key masked
  const { data: savedConfig, isLoading } = useQuery({
    queryKey: ['byok-config', showApiKey],
    queryFn: () => fetchBYOKConfig(showApiKey),
  });

  useEffect(() => {
    if (savedConfig) {
      setConfig(savedConfig);
    }
  }, [savedConfig]);

  // Save mutation
  const saveMutation = useMutation({
    mutationFn: saveBYOKConfig,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['byok-config'] });
      showToast('success', 'Success', 'BYOK configuration saved successfully');
    },
    onError: (error: Error) => {
      showToast('error', 'Error', error.message || 'Failed to save configuration');
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: deleteBYOKConfig,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['byok-config'] });
      setConfig({ provider: 'openai', api_key: '', enabled: false });
      setShowDeleteConfirm(false);
      showToast('success', 'Deleted', 'BYOK configuration removed');
    },
    onError: (error: Error) => {
      showToast('error', 'Error', error.message || 'Failed to delete configuration');
    },
  });

  // Test connection mutation
  const testMutation = useMutation({
    mutationFn: testBYOKConnection,
    onSuccess: (result) => {
      if (result.success) {
        showToast('success', 'Connected', result.message);
      } else {
        showToast('error', 'Connection Failed', result.message);
      }
    },
    onError: (error: Error) => {
      showToast('error', 'Error', error.message || 'Failed to test connection');
    },
  });

  const handleSave = () => {
    if (!config.api_key.trim() && config.enabled) {
      showToast('error', 'Error', 'API key is required when BYOK is enabled');
      return;
    }
    if (config.provider === 'custom' && !config.base_url?.trim()) {
      showToast('error', 'Error', 'Base URL is required for custom provider');
      return;
    }
    saveMutation.mutate(config);
  };

  const handleTestConnection = () => {
    if (!config.api_key.trim()) {
      showToast('error', 'Error', 'Enter an API key to test');
      return;
    }
    testMutation.mutate({
      provider: config.provider,
      api_key: config.api_key,
      base_url: config.base_url,
      model: config.model,
    });
  };

  const selectedProvider = PROVIDERS.find((p) => p.value === config.provider);

  if (isLoading) {
    return <div className="text-center py-12">Loading configuration...</div>;
  }

  return (
    <div className="grid gap-6 md:grid-cols-3">
      {/* Configuration Form */}
      <Card className="md:col-span-2">
        <CardHeader>
          <CardTitle className="flex items-center">
            <Key className="w-5 h-5 mr-2" />
            Configuration
          </CardTitle>
          <CardDescription>
            Set up your AI provider API key to use your own credits
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Provider Selection */}
          <div>
            <Label>AI Provider</Label>
            <div className="grid grid-cols-2 gap-3 mt-2">
              {PROVIDERS.map((provider) => (
                <div
                  key={provider.value}
                  className={`p-4 border rounded-lg cursor-pointer transition-colors ${
                    config.provider === provider.value
                      ? 'border-primary bg-primary/5'
                      : 'hover:bg-accent'
                  }`}
                  onClick={() =>
                    setConfig({
                      ...config,
                      provider: provider.value,
                      base_url: provider.baseUrl || config.base_url,
                      model: undefined,
                    })
                  }
                >
                  <div className="font-medium">{provider.label}</div>
                  <div className="text-sm text-muted-foreground mt-1">
                    {provider.description}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Model Selection */}
          {selectedProvider && selectedProvider.models.length > 0 && (
            <div>
              <Label>Model</Label>
              <Select
                value={config.model || ''}
                onValueChange={(value) => setConfig({ ...config, model: value })}
              >
                <SelectTrigger className="mt-2">
                  <SelectValue placeholder="Select a model (optional)" />
                </SelectTrigger>
                <SelectContent>
                  {selectedProvider.models.map((model) => (
                    <SelectItem key={model.value} value={model.value}>
                      {model.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-sm text-muted-foreground mt-1">
                Optional — defaults to provider&apos;s recommended model
              </p>
            </div>
          )}

          {/* Custom model free-text for custom provider */}
          {config.provider === 'custom' && (
            <div>
              <Label htmlFor="custom-model">Model Name</Label>
              <Input
                id="custom-model"
                placeholder="e.g., gpt-4o, llama-3-70b"
                value={config.model || ''}
                onChange={(e) => setConfig({ ...config, model: e.target.value })}
                className="mt-2"
              />
              <p className="text-sm text-muted-foreground mt-1">
                Model identifier for your OpenAI-compatible API
              </p>
            </div>
          )}

          {/* API Key */}
          <div>
            <Label htmlFor="api-key">API Key</Label>
            <div className="relative mt-2">
              <Input
                id="api-key"
                type={showApiKey ? 'text' : 'password'}
                placeholder="sk-..."
                value={config.api_key}
                onChange={(e) => setConfig({ ...config, api_key: e.target.value })}
                className="pr-10"
              />
              <Button
                variant="ghost"
                size="sm"
                className="absolute right-2 top-1/2 -translate-y-1/2"
                onClick={() => setShowApiKey(!showApiKey)}
              >
                {showApiKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </Button>
            </div>
            {selectedProvider?.docsUrl && (
              <div className="mt-1">
                <a
                  href={selectedProvider.docsUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-primary hover:underline flex items-center"
                >
                  View API documentation
                  <ExternalLink className="w-3 h-3 ml-1" />
                </a>
              </div>
            )}
          </div>

          {/* Base URL (for custom provider) */}
          {config.provider === 'custom' && (
            <div>
              <Label htmlFor="base-url">Base URL</Label>
              <Input
                id="base-url"
                type="url"
                placeholder="https://api.example.com/v1"
                value={config.base_url || ''}
                onChange={(e) => setConfig({ ...config, base_url: e.target.value })}
                className="mt-2"
              />
              <p className="text-sm text-muted-foreground mt-1">
                Base URL for your OpenAI-compatible API
              </p>
            </div>
          )}

          {/* Enable Toggle */}
          <div className="flex items-center justify-between p-4 border rounded-lg">
            <div>
              <div className="font-medium">Enable BYOK</div>
              <div className="text-sm text-muted-foreground">
                Use your own API key for AI features
              </div>
            </div>
            <Button
              variant={config.enabled ? 'default' : 'outline'}
              onClick={() => setConfig({ ...config, enabled: !config.enabled })}
            >
              {config.enabled ? (
                <>
                  <CheckCircle2 className="w-4 h-4 mr-2" />
                  Enabled
                </>
              ) : (
                <>
                  <XCircle className="w-4 h-4 mr-2" />
                  Disabled
                </>
              )}
            </Button>
          </div>

          {/* Action Buttons */}
          <div className="flex gap-3">
            <Button onClick={handleSave} disabled={saveMutation.isPending} className="flex-1">
              <Save className="w-4 h-4 mr-2" />
              {saveMutation.isPending ? 'Saving...' : 'Save Configuration'}
            </Button>
            <Button
              variant="outline"
              onClick={handleTestConnection}
              disabled={testMutation.isPending || !config.api_key.trim()}
            >
              {testMutation.isPending ? (
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
              ) : (
                <Zap className="w-4 h-4 mr-2" />
              )}
              Test
            </Button>
          </div>

          {/* Delete Configuration */}
          {savedConfig?.api_key && (
            <div className="border-t pt-4">
              {showDeleteConfirm ? (
                <div className="flex items-center gap-3">
                  <p className="text-sm text-muted-foreground flex-1">
                    This will permanently remove your stored API key.
                  </p>
                  <Button
                    variant="destructive"
                    size="sm"
                    onClick={() => deleteMutation.mutate()}
                    disabled={deleteMutation.isPending}
                  >
                    {deleteMutation.isPending ? 'Deleting...' : 'Confirm Delete'}
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setShowDeleteConfirm(false)}
                  >
                    Cancel
                  </Button>
                </div>
              ) : (
                <Button
                  variant="ghost"
                  className="text-destructive hover:text-destructive"
                  onClick={() => setShowDeleteConfirm(true)}
                >
                  <Trash2 className="w-4 h-4 mr-2" />
                  Remove Configuration
                </Button>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Info Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center">
            <Info className="w-5 h-5 mr-2" />
            About BYOK
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <h3 className="font-semibold mb-2">Free Tier</h3>
            <p className="text-sm text-muted-foreground">
              On the Free plan, BYOK is required to use AI features. Connect your own API key to
              get started.
            </p>
          </div>
          <div>
            <h3 className="font-semibold mb-2">Paid Plans</h3>
            <p className="text-sm text-muted-foreground">
              Pro and Team plans include hosted AI credits, but you can still use BYOK for
              additional capacity or custom models.
            </p>
          </div>
          <div>
            <h3 className="font-semibold mb-2">Security</h3>
            <p className="text-sm text-muted-foreground">
              Your API keys are encrypted with AES-256-GCM and stored securely. They are only used
              for AI requests you make.
            </p>
          </div>
          <div className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-3">
            <div className="flex items-start">
              <AlertCircle className="w-4 h-4 mr-2 text-yellow-600 dark:text-yellow-400 mt-0.5" />
              <p className="text-sm text-yellow-800 dark:text-yellow-200">
                Keep your API keys secure. Never share them publicly or commit them to version
                control.
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

// ────────────────────────────────────────────────────────────────────────────
// AI Features Tab
// ────────────────────────────────────────────────────────────────────────────

function AiFeaturesTab({ orgId }: { orgId: string }) {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [settings, setSettings] = useState<OrgAiSettings | null>(null);

  const { data: fetchedSettings, isLoading } = useQuery({
    queryKey: ['org-ai-settings', orgId],
    queryFn: () => fetchOrgAiSettings(orgId),
    enabled: !!orgId,
  });

  useEffect(() => {
    if (fetchedSettings) {
      setSettings(fetchedSettings);
    }
  }, [fetchedSettings]);

  const saveMutation = useMutation({
    mutationFn: (s: OrgAiSettings) => saveOrgAiSettings(orgId, s),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['org-ai-settings', orgId] });
      showToast('success', 'Saved', 'AI settings updated');
    },
    onError: (error: Error) => {
      showToast('error', 'Error', error.message || 'Failed to save AI settings');
    },
  });

  if (isLoading || !settings) {
    return <div className="text-center py-12">Loading AI settings...</div>;
  }

  const toggleFlag = (key: keyof OrgAiSettings['feature_flags']) => {
    setSettings({
      ...settings,
      feature_flags: {
        ...settings.feature_flags,
        [key]: !settings.feature_flags[key],
      },
    });
  };

  return (
    <div className="grid gap-6 md:grid-cols-2">
      {/* Feature Flags */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center">
            <Settings2 className="w-5 h-5 mr-2" />
            AI Feature Flags
          </CardTitle>
          <CardDescription>Enable or disable individual AI features</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {AI_FEATURES.map((feature) => (
            <div
              key={feature.key}
              className="flex items-center justify-between p-3 border rounded-lg"
            >
              <div className="flex-1 mr-4">
                <div className="font-medium text-sm">{feature.label}</div>
                <div className="text-xs text-muted-foreground">{feature.description}</div>
              </div>
              <Switch
                checked={settings.feature_flags[feature.key]}
                onCheckedChange={() => toggleFlag(feature.key)}
              />
            </div>
          ))}
        </CardContent>
      </Card>

      {/* Rate Limits */}
      <Card>
        <CardHeader>
          <CardTitle>Rate Limits</CardTitle>
          <CardDescription>
            Control AI usage limits per workspace
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <Label htmlFor="daily-limit">Max AI calls per workspace per day</Label>
            <Input
              id="daily-limit"
              type="number"
              min={0}
              value={settings.max_ai_calls_per_workspace_per_day}
              onChange={(e) =>
                setSettings({
                  ...settings,
                  max_ai_calls_per_workspace_per_day: parseInt(e.target.value) || 0,
                })
              }
              className="mt-2"
            />
          </div>
          <div>
            <Label htmlFor="monthly-limit">Max AI calls per workspace per month</Label>
            <Input
              id="monthly-limit"
              type="number"
              min={0}
              value={settings.max_ai_calls_per_workspace_per_month}
              onChange={(e) =>
                setSettings({
                  ...settings,
                  max_ai_calls_per_workspace_per_month: parseInt(e.target.value) || 0,
                })
              }
              className="mt-2"
            />
          </div>
          <Button
            onClick={() => saveMutation.mutate(settings)}
            disabled={saveMutation.isPending}
            className="w-full mt-4"
          >
            <Save className="w-4 h-4 mr-2" />
            {saveMutation.isPending ? 'Saving...' : 'Save AI Settings'}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}

// ────────────────────────────────────────────────────────────────────────────
// Usage Tab
// ────────────────────────────────────────────────────────────────────────────

function UsageTab({ orgId }: { orgId: string }) {
  const { data: usage, isLoading } = useQuery({
    queryKey: ['org-usage', orgId],
    queryFn: () => fetchOrgUsage(orgId),
    enabled: !!orgId,
  });

  if (isLoading) {
    return <div className="text-center py-12">Loading usage data...</div>;
  }

  if (!usage) {
    return <div className="text-center py-12 text-muted-foreground">No usage data available</div>;
  }

  const stats = [
    { label: 'AI Tokens Used', value: usage.total_ai_tokens.toLocaleString(), icon: Zap },
    { label: 'Total Requests', value: usage.total_requests.toLocaleString(), icon: BarChart3 },
    {
      label: 'Storage Used',
      value: `${usage.total_storage_gb.toFixed(2)} GB`,
      icon: ScrollText,
    },
    { label: 'Hosted Mocks', value: usage.hosted_mocks_count.toString(), icon: Settings2 },
    { label: 'Plugins Published', value: usage.plugins_published.toString(), icon: Key },
    { label: 'API Tokens', value: usage.api_tokens_count.toString(), icon: Key },
  ];

  return (
    <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-3">
        {stats.map((stat) => (
          <Card key={stat.label}>
            <CardContent className="pt-6">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-muted-foreground">{stat.label}</p>
                  <p className="text-2xl font-bold">{stat.value}</p>
                </div>
                <stat.icon className="w-8 h-8 text-muted-foreground/30" />
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
      <div className="text-sm text-muted-foreground">
        For detailed usage breakdown, visit the{' '}
        <a href="/usage" className="text-primary hover:underline">
          Usage Dashboard
        </a>
        .
      </div>
    </div>
  );
}

// ────────────────────────────────────────────────────────────────────────────
// Audit Log Tab
// ────────────────────────────────────────────────────────────────────────────

const AUDIT_PAGE_SIZE = 10;

function AuditLogTab({ orgId }: { orgId: string }) {
  const [page, setPage] = useState(0);
  const { data: logs, isLoading } = useQuery({
    queryKey: ['byok-audit-logs', orgId],
    queryFn: () => fetchAuditLogs(orgId),
    enabled: !!orgId,
  });

  if (isLoading) {
    return <div className="text-center py-12">Loading audit logs...</div>;
  }

  if (!logs || logs.length === 0) {
    return (
      <Card>
        <CardContent className="pt-6">
          <div className="text-center py-8 text-muted-foreground">
            No BYOK configuration changes recorded yet.
          </div>
        </CardContent>
      </Card>
    );
  }

  const totalPages = Math.ceil(logs.length / AUDIT_PAGE_SIZE);
  const pagedLogs = logs.slice(page * AUDIT_PAGE_SIZE, (page + 1) * AUDIT_PAGE_SIZE);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center">
          <ScrollText className="w-5 h-5 mr-2" />
          BYOK Configuration History
        </CardTitle>
        <CardDescription>
          Recent changes to your BYOK settings ({logs.length} total)
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          {pagedLogs.map((log) => (
            <div key={log.id} className="flex items-start gap-3 p-3 border rounded-lg">
              <div
                className={`mt-1 w-2 h-2 rounded-full shrink-0 ${
                  log.event_type === 'byok_config_deleted' ? 'bg-red-500' : 'bg-green-500'
                }`}
              />
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium">{log.description}</p>
                {log.metadata && (
                  <div className="flex gap-2 mt-1 flex-wrap">
                    {log.metadata.provider && (
                      <Badge variant="secondary">
                        {String(log.metadata.provider)}
                      </Badge>
                    )}
                    {log.metadata.enabled !== undefined && (
                      <Badge variant={log.metadata.enabled ? 'default' : 'outline'}>
                        {log.metadata.enabled ? 'Enabled' : 'Disabled'}
                      </Badge>
                    )}
                  </div>
                )}
                <p className="text-xs text-muted-foreground mt-1">
                  {new Date(log.created_at).toLocaleString()}
                  {log.ip_address && ` \u00b7 ${log.ip_address}`}
                </p>
              </div>
            </div>
          ))}
        </div>

        {/* Pagination */}
        {totalPages > 1 && (
          <div className="flex items-center justify-between mt-4 pt-4 border-t">
            <p className="text-sm text-muted-foreground">
              Page {page + 1} of {totalPages}
            </p>
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPage((p) => Math.max(0, p - 1))}
                disabled={page === 0}
              >
                Previous
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
                disabled={page >= totalPages - 1}
              >
                Next
              </Button>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

// ────────────────────────────────────────────────────────────────────────────
// Main Page
// ────────────────────────────────────────────────────────────────────────────

export function BYOKConfigPage() {
  const { data: organizations, isLoading: orgsLoading } = useQuery({
    queryKey: ['organizations'],
    queryFn: fetchOrganizations,
  });

  const orgId = organizations?.[0]?.id;

  return (
    <div className="container mx-auto p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Bring Your Own Key (BYOK)</h1>
        <p className="text-muted-foreground mt-2">
          Configure your own AI provider API keys, manage AI features, and monitor usage
        </p>
      </div>

      <Tabs defaultValue="configuration">
        <TabsList>
          <TabsTrigger value="configuration">
            <Key className="w-4 h-4 mr-2" />
            Configuration
          </TabsTrigger>
          <TabsTrigger value="ai-features">
            <Settings2 className="w-4 h-4 mr-2" />
            AI Features
          </TabsTrigger>
          <TabsTrigger value="usage">
            <BarChart3 className="w-4 h-4 mr-2" />
            Usage
          </TabsTrigger>
          <TabsTrigger value="audit">
            <ScrollText className="w-4 h-4 mr-2" />
            Audit Log
          </TabsTrigger>
        </TabsList>

        <TabsContent value="configuration">
          <ConfigurationTab />
        </TabsContent>

        <TabsContent value="ai-features">
          {orgsLoading ? (
            <div className="text-center py-12">Loading...</div>
          ) : orgId ? (
            <AiFeaturesTab orgId={orgId} />
          ) : (
            <div className="text-center py-12 text-muted-foreground">
              No organization found. Please create one first.
            </div>
          )}
        </TabsContent>

        <TabsContent value="usage">
          {orgsLoading ? (
            <div className="text-center py-12">Loading...</div>
          ) : orgId ? (
            <UsageTab orgId={orgId} />
          ) : (
            <div className="text-center py-12 text-muted-foreground">
              No organization found.
            </div>
          )}
        </TabsContent>

        <TabsContent value="audit">
          {orgsLoading ? (
            <div className="text-center py-12">Loading...</div>
          ) : orgId ? (
            <AuditLogTab orgId={orgId} />
          ) : (
            <div className="text-center py-12 text-muted-foreground">
              No organization found.
            </div>
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}
