import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/Badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
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
} from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';

// Types
interface BYOKConfig {
  provider: 'openai' | 'anthropic' | 'together' | 'fireworks' | 'custom';
  api_key: string;
  base_url?: string;
  enabled: boolean;
}

// API base URL - adjust based on your setup
const API_BASE = '/api/v1';

async function fetchBYOKConfig(): Promise<BYOKConfig> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/settings/byok`, {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    // If 404, return default config
    if (response.status === 404) {
      return {
        provider: 'openai',
        api_key: '',
        enabled: false,
      };
    }
    throw new Error('Failed to fetch BYOK config');
  }
  return response.json();
}

async function saveBYOKConfig(config: BYOKConfig): Promise<void> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/settings/byok`, {
    method: 'PUT',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(config),
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || 'Failed to save BYOK config');
  }
}

const PROVIDERS = [
  {
    value: 'openai',
    label: 'OpenAI',
    description: 'Use OpenAI API (GPT-4, GPT-3.5, etc.)',
    baseUrl: 'https://api.openai.com/v1',
    docsUrl: 'https://platform.openai.com/docs',
  },
  {
    value: 'anthropic',
    label: 'Anthropic',
    description: 'Use Anthropic API (Claude)',
    baseUrl: 'https://api.anthropic.com/v1',
    docsUrl: 'https://docs.anthropic.com',
  },
  {
    value: 'together',
    label: 'Together AI',
    description: 'Use Together AI for open-source models',
    baseUrl: 'https://api.together.xyz/v1',
    docsUrl: 'https://docs.together.ai',
  },
  {
    value: 'fireworks',
    label: 'Fireworks AI',
    description: 'Use Fireworks AI for fast inference',
    baseUrl: 'https://api.fireworks.ai/inference/v1',
    docsUrl: 'https://docs.fireworks.ai',
  },
  {
    value: 'custom',
    label: 'Custom',
    description: 'Use a custom OpenAI-compatible API',
    baseUrl: '',
    docsUrl: '',
  },
];

export function BYOKConfigPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [showApiKey, setShowApiKey] = useState(false);
  const [config, setConfig] = useState<BYOKConfig>({
    provider: 'openai',
    api_key: '',
    enabled: false,
  });

  // Fetch BYOK config
  const { data: savedConfig, isLoading } = useQuery({
    queryKey: ['byok-config'],
    queryFn: fetchBYOKConfig,
    onSuccess: (data) => {
      setConfig(data);
    },
  });

  // Save config mutation
  const saveMutation = useMutation({
    mutationFn: saveBYOKConfig,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['byok-config'] });
      showToast({
        title: 'Success',
        description: 'BYOK configuration saved successfully',
      });
    },
    onError: (error: Error) => {
      showToast({
        title: 'Error',
        description: error.message || 'Failed to save configuration',
        variant: 'destructive',
      });
    },
  });

  const handleSave = () => {
    if (!config.api_key.trim() && config.enabled) {
      showToast({
        title: 'Error',
        description: 'API key is required when BYOK is enabled',
        variant: 'destructive',
      });
      return;
    }
    saveMutation.mutate(config);
  };

  const selectedProvider = PROVIDERS.find((p) => p.value === config.provider);

  return (
    <div className="container mx-auto p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Bring Your Own Key (BYOK)</h1>
        <p className="text-muted-foreground mt-2">
          Configure your own AI provider API keys for Free tier or additional capacity
        </p>
      </div>

      {isLoading ? (
        <div className="text-center py-12">Loading configuration...</div>
      ) : (
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
                          provider: provider.value as BYOKConfig['provider'],
                          base_url: provider.baseUrl || config.base_url,
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

              <Button
                onClick={handleSave}
                disabled={saveMutation.isPending}
                className="w-full"
              >
                <Save className="w-4 h-4 mr-2" />
                {saveMutation.isPending ? 'Saving...' : 'Save Configuration'}
              </Button>
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
                  On the Free plan, BYOK is required to use AI features. Connect your own API key
                  to get started.
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
                  Your API keys are encrypted and stored securely. They are only used for AI
                  requests you make.
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
      )}
    </div>
  );
}
