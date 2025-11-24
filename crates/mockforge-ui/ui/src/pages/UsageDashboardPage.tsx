import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { Badge } from '@/components/ui/Badge';
import {
  TrendingUp,
  HardDrive,
  Zap,
  Activity,
  Calendar,
  AlertCircle,
} from 'lucide-react';

// Types
interface UsageResponse {
  org_id: string;
  period_start: string;
  period_end: string;
  usage: {
    requests: { used: number; limit: number; unit: string };
    storage: { used: number; limit: number; unit: string };
    egress: { used: number; limit: number; unit: string };
    ai_tokens: { used: number; limit: number; unit: string };
  };
  limits: Record<string, unknown>;
  plan: string;
}

interface UsageHistoryResponse {
  org_id: string;
  history: Array<{
    period_start: string;
    period_end: string;
    requests: number;
    egress_bytes: number;
    storage_bytes: number;
    ai_tokens_used: number;
  }>;
}

// API base URL
const API_BASE = '/api/v1';

async function fetchUsage(): Promise<UsageResponse> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/usage`, {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    throw new Error('Failed to fetch usage');
  }
  return response.json();
}

async function fetchUsageHistory(): Promise<UsageHistoryResponse> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/usage/history`, {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    throw new Error('Failed to fetch usage history');
  }
  return response.json();
}

const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
};

const formatNumber = (num: number) => {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toString();
};

const getUsagePercentage = (used: number, limit: number) => {
  if (limit <= 0) return 0;
  return Math.min((used / limit) * 100, 100);
};

const getUsageColor = (percentage: number) => {
  if (percentage > 90) return 'bg-red-500';
  if (percentage > 75) return 'bg-yellow-500';
  return 'bg-green-500';
};

export function UsageDashboardPage() {
  const { data: usage, isLoading: usageLoading } = useQuery({
    queryKey: ['usage'],
    queryFn: fetchUsage,
  });

  const { data: history, isLoading: historyLoading } = useQuery({
    queryKey: ['usage-history'],
    queryFn: fetchUsageHistory,
  });

  if (usageLoading) {
    return (
      <div className="container mx-auto p-6">
        <div className="text-center py-12">Loading usage data...</div>
      </div>
    );
  }

  if (!usage) {
    return (
      <div className="container mx-auto p-6">
        <div className="text-center py-12">Failed to load usage data</div>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Usage Dashboard</h1>
        <p className="text-muted-foreground mt-2">
          Monitor your organization's usage and limits
        </p>
      </div>

      {/* Period Info */}
      <Card>
        <CardContent className="p-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <Calendar className="w-4 h-4 text-muted-foreground" />
              <span className="text-sm text-muted-foreground">
                Current Period: {new Date(usage.period_start).toLocaleDateString()} -{' '}
                {new Date(usage.period_end).toLocaleDateString()}
              </span>
            </div>
            <Badge className="capitalize">{usage.plan} Plan</Badge>
          </div>
        </CardContent>
      </Card>

      <Tabs defaultValue="current" className="space-y-4">
        <TabsList>
          <TabsTrigger value="current">Current Usage</TabsTrigger>
          <TabsTrigger value="history">History</TabsTrigger>
        </TabsList>

        {/* Current Usage Tab */}
        <TabsContent value="current" className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            {/* Requests */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center">
                  <TrendingUp className="w-5 h-5 mr-2" />
                  API Requests
                </CardTitle>
                <CardDescription>Monthly request usage</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <div className="flex justify-between text-sm mb-2">
                    <span>Used</span>
                    <span className="font-semibold">
                      {formatNumber(usage.usage.requests.used)} /{' '}
                      {formatNumber(usage.usage.requests.limit)}
                    </span>
                  </div>
                  <div className="w-full bg-secondary rounded-full h-3">
                    <div
                      className={`h-3 rounded-full transition-all ${getUsageColor(
                        getUsagePercentage(usage.usage.requests.used, usage.usage.requests.limit)
                      )}`}
                      style={{
                        width: `${getUsagePercentage(
                          usage.usage.requests.used,
                          usage.usage.requests.limit
                        )}%`,
                      }}
                    />
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    {(
                      usage.usage.requests.limit - usage.usage.requests.used
                    ).toLocaleString()}{' '}
                    requests remaining
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Storage */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center">
                  <HardDrive className="w-5 h-5 mr-2" />
                  Storage
                </CardTitle>
                <CardDescription>Storage usage</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <div className="flex justify-between text-sm mb-2">
                    <span>Used</span>
                    <span className="font-semibold">
                      {formatBytes(usage.usage.storage.used)} /{' '}
                      {formatBytes(usage.usage.storage.limit)}
                    </span>
                  </div>
                  <div className="w-full bg-secondary rounded-full h-3">
                    <div
                      className={`h-3 rounded-full transition-all ${getUsageColor(
                        getUsagePercentage(usage.usage.storage.used, usage.usage.storage.limit)
                      )}`}
                      style={{
                        width: `${getUsagePercentage(
                          usage.usage.storage.used,
                          usage.usage.storage.limit
                        )}%`,
                      }}
                    />
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    {formatBytes(usage.usage.storage.limit - usage.usage.storage.used)} remaining
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Egress */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center">
                  <Activity className="w-5 h-5 mr-2" />
                  Data Egress
                </CardTitle>
                <CardDescription>Data transfer usage</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <div className="flex justify-between text-sm mb-2">
                    <span>Used</span>
                    <span className="font-semibold">
                      {formatBytes(usage.usage.egress.used)}
                    </span>
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    {usage.usage.egress.limit === -1
                      ? 'Unlimited'
                      : `${formatBytes(usage.usage.egress.limit - usage.usage.egress.used)} remaining`}
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* AI Tokens */}
            {usage.usage.ai_tokens.limit > 0 && (
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center">
                    <Zap className="w-5 h-5 mr-2" />
                    AI Tokens
                  </CardTitle>
                  <CardDescription>AI token usage</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div>
                    <div className="flex justify-between text-sm mb-2">
                      <span>Used</span>
                      <span className="font-semibold">
                        {formatNumber(usage.usage.ai_tokens.used)} /{' '}
                        {formatNumber(usage.usage.ai_tokens.limit)}
                      </span>
                    </div>
                    <div className="w-full bg-secondary rounded-full h-3">
                      <div
                        className={`h-3 rounded-full transition-all ${getUsageColor(
                          getUsagePercentage(
                            usage.usage.ai_tokens.used,
                            usage.usage.ai_tokens.limit
                          )
                        )}`}
                        style={{
                          width: `${getUsagePercentage(
                            usage.usage.ai_tokens.used,
                            usage.usage.ai_tokens.limit
                          )}%`,
                        }}
                      />
                    </div>
                    <div className="text-xs text-muted-foreground mt-1">
                      {(
                        usage.usage.ai_tokens.limit - usage.usage.ai_tokens.used
                      ).toLocaleString()}{' '}
                      tokens remaining
                    </div>
                  </div>
                </CardContent>
              </Card>
            )}
          </div>

          {/* Usage Warnings */}
          {Object.values(usage.usage).some(
            (metric) =>
              metric.limit > 0 &&
              getUsagePercentage(metric.used, metric.limit) > 75
          ) && (
            <Card className="border-yellow-500">
              <CardContent className="p-4">
                <div className="flex items-start space-x-3">
                  <AlertCircle className="w-5 h-5 text-yellow-500 mt-0.5" />
                  <div>
                    <h3 className="font-semibold mb-1">Usage Warning</h3>
                    <p className="text-sm text-muted-foreground">
                      You're approaching your plan limits. Consider upgrading to avoid service
                      interruptions.
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>
          )}
        </TabsContent>

        {/* History Tab */}
        <TabsContent value="history" className="space-y-4">
          {historyLoading ? (
            <div className="text-center py-12">Loading history...</div>
          ) : history && history.history.length > 0 ? (
            <div className="space-y-4">
              {history.history.map((period, index) => (
                <Card key={index}>
                  <CardHeader>
                    <CardTitle className="text-lg">
                      {new Date(period.period_start).toLocaleDateString('en-US', {
                        month: 'long',
                        year: 'numeric',
                      })}
                    </CardTitle>
                    <CardDescription>
                      {new Date(period.period_start).toLocaleDateString()} -{' '}
                      {new Date(period.period_end).toLocaleDateString()}
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                      <div>
                        <div className="text-sm text-muted-foreground">Requests</div>
                        <div className="text-lg font-semibold">
                          {formatNumber(period.requests)}
                        </div>
                      </div>
                      <div>
                        <div className="text-sm text-muted-foreground">Storage</div>
                        <div className="text-lg font-semibold">
                          {formatBytes(period.storage_bytes)}
                        </div>
                      </div>
                      <div>
                        <div className="text-sm text-muted-foreground">Egress</div>
                        <div className="text-lg font-semibold">
                          {formatBytes(period.egress_bytes)}
                        </div>
                      </div>
                      {usage.usage.ai_tokens.limit > 0 && (
                        <div>
                          <div className="text-sm text-muted-foreground">AI Tokens</div>
                          <div className="text-lg font-semibold">
                            {formatNumber(period.ai_tokens_used)}
                          </div>
                        </div>
                      )}
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          ) : (
            <Card>
              <CardContent className="p-12 text-center">
                <Calendar className="w-12 h-12 mx-auto text-muted-foreground mb-4" />
                <h3 className="text-lg font-semibold mb-2">No History Available</h3>
                <p className="text-muted-foreground">
                  Usage history will appear here as you use the service
                </p>
              </CardContent>
            </Card>
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}
