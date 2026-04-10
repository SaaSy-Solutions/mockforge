import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/Skeleton';
import {
  TrendingUp,
  HardDrive,
  Zap,
  Activity,
  Calendar,
  AlertCircle,
  RefreshCw,
  ArrowUpRight,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';
import { authenticatedFetch } from '@/utils/apiClient';

// Types
interface UsageResponse {
  org_id: string;
  period_start: string;
  period_end: string;
  usage: {
    requests: UsageMetric;
    storage: UsageMetric;
    egress: UsageMetric;
    ai_tokens: UsageMetric;
  };
  plan: string;
}

interface UsageMetric {
  used: number;
  limit: number;
  unit: string;
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

const HISTORY_PAGE_SIZE = 6;

async function fetchUsage(): Promise<UsageResponse> {
  const response = await authenticatedFetch(`${API_BASE}/usage`);
  if (!response.ok) {
    throw new Error('Failed to fetch usage');
  }
  return response.json();
}

async function fetchUsageHistory(): Promise<UsageHistoryResponse> {
  const response = await authenticatedFetch(`${API_BASE}/usage/history`);
  if (!response.ok) {
    throw new Error('Failed to fetch usage history');
  }
  return response.json();
}

// Use SI units (base-1000) to match backend which stores limits in SI bytes
const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 B';
  const k = 1000;
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

const METRIC_LABELS: Record<string, string> = {
  requests: 'API Requests',
  storage: 'Storage',
  egress: 'Data Egress',
  ai_tokens: 'AI Tokens',
};

function getHighUsageMetrics(usage: UsageResponse['usage']): string[] {
  return Object.entries(usage)
    .filter(([, metric]) => metric.limit > 0 && getUsagePercentage(metric.used, metric.limit) > 75)
    .map(([key]) => METRIC_LABELS[key] || key);
}

/** Skeleton placeholder for a usage metric card */
function UsageCardSkeleton() {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center space-x-2">
          <Skeleton width={20} height={20} />
          <Skeleton width="40%" height={20} />
        </div>
        <Skeleton width="60%" height={14} className="mt-1" />
      </CardHeader>
      <CardContent className="space-y-4">
        <div>
          <div className="flex justify-between mb-2">
            <Skeleton width={40} height={14} />
            <Skeleton width={80} height={14} />
          </div>
          <Skeleton width="100%" height={12} className="rounded-full" />
          <Skeleton width="50%" height={10} className="mt-1" />
        </div>
      </CardContent>
    </Card>
  );
}

export function UsageDashboardPage() {
  const [historyPage, setHistoryPage] = useState(0);

  const {
    data: usage,
    isLoading: usageLoading,
    isError: usageError,
    error: usageErrorDetail,
    refetch: refetchUsage,
  } = useQuery({
    queryKey: ['usage'],
    queryFn: fetchUsage,
    staleTime: 60_000,
    refetchInterval: 60_000,
  });

  const {
    data: history,
    isLoading: historyLoading,
    isError: historyError,
    refetch: refetchHistory,
  } = useQuery({
    queryKey: ['usage-history'],
    queryFn: fetchUsageHistory,
    staleTime: 60_000,
    refetchInterval: 60_000,
  });

  if (usageLoading) {
    return (
      <div className="container mx-auto p-6 space-y-6">
        <div>
          <Skeleton width="50%" height={32} />
          <Skeleton width="70%" height={16} className="mt-2" />
        </div>
        <Card>
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <Skeleton width="40%" height={16} />
              <Skeleton width={80} height={24} />
            </div>
          </CardContent>
        </Card>
        <div className="grid gap-4 md:grid-cols-2">
          <UsageCardSkeleton />
          <UsageCardSkeleton />
          <UsageCardSkeleton />
        </div>
      </div>
    );
  }

  if (usageError || !usage) {
    return (
      <div className="container mx-auto p-6">
        <div className="text-center py-12 space-y-4">
          <AlertCircle className="w-12 h-12 mx-auto text-red-500" />
          <h2 className="text-lg font-semibold">Failed to load usage data</h2>
          <p className="text-sm text-muted-foreground">
            {usageErrorDetail instanceof Error ? usageErrorDetail.message : 'An unexpected error occurred'}
          </p>
          <Button variant="outline" onClick={() => refetchUsage()}>
            <RefreshCw className="w-4 h-4 mr-2" />
            Retry
          </Button>
        </div>
      </div>
    );
  }

  const highUsageMetrics = getHighUsageMetrics(usage.usage);

  // Pagination for history
  const totalHistory = history?.history.length ?? 0;
  const totalPages = Math.max(1, Math.ceil(totalHistory / HISTORY_PAGE_SIZE));
  const pagedHistory = history?.history.slice(
    historyPage * HISTORY_PAGE_SIZE,
    (historyPage + 1) * HISTORY_PAGE_SIZE,
  ) ?? [];

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
            <UsageCard
              icon={<TrendingUp className="w-5 h-5 mr-2" />}
              title="API Requests"
              description="Monthly request usage"
              metric={usage.usage.requests}
              formatValue={formatNumber}
              remainingLabel="requests"
            />

            {/* Storage */}
            <UsageCard
              icon={<HardDrive className="w-5 h-5 mr-2" />}
              title="Storage"
              description="Storage usage"
              metric={usage.usage.storage}
              formatValue={formatBytes}
            />

            {/* Egress */}
            <UsageCard
              icon={<Activity className="w-5 h-5 mr-2" />}
              title="Data Egress"
              description="Data transfer usage"
              metric={usage.usage.egress}
              formatValue={formatBytes}
            />

            {/* AI Tokens - show if limit > 0 OR if there's actual usage */}
            {(usage.usage.ai_tokens.limit > 0 || usage.usage.ai_tokens.used > 0) && (
              <UsageCard
                icon={<Zap className="w-5 h-5 mr-2" />}
                title="AI Tokens"
                description="AI token usage"
                metric={usage.usage.ai_tokens}
                formatValue={formatNumber}
                remainingLabel="tokens"
              />
            )}
          </div>

          {/* Usage Warnings */}
          {highUsageMetrics.length > 0 && (
            <Card className="border-yellow-500">
              <CardContent className="p-4">
                <div className="flex items-start space-x-3">
                  <AlertCircle className="w-5 h-5 text-yellow-500 mt-0.5" />
                  <div className="flex-1">
                    <h3 className="font-semibold mb-1">Usage Warning</h3>
                    <p className="text-sm text-muted-foreground">
                      {highUsageMetrics.join(', ')}{' '}
                      {highUsageMetrics.length === 1 ? 'is' : 'are'} approaching{' '}
                      {highUsageMetrics.length === 1 ? 'its' : 'their'} plan limit.
                    </p>
                    <a
                      href="/billing"
                      className="inline-flex items-center mt-2 text-sm font-medium text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
                    >
                      Upgrade plan
                      <ArrowUpRight className="w-3 h-3 ml-1" />
                    </a>
                  </div>
                </div>
              </CardContent>
            </Card>
          )}
        </TabsContent>

        {/* History Tab */}
        <TabsContent value="history" className="space-y-4">
          {historyLoading ? (
            <div className="space-y-4">
              {[0, 1, 2].map((i) => (
                <Card key={i}>
                  <CardHeader>
                    <Skeleton width="30%" height={20} />
                    <Skeleton width="50%" height={14} className="mt-1" />
                  </CardHeader>
                  <CardContent>
                    <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                      {[0, 1, 2, 3].map((j) => (
                        <div key={j}>
                          <Skeleton width="60%" height={12} />
                          <Skeleton width="40%" height={20} className="mt-1" />
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          ) : historyError ? (
            <div className="text-center py-12 space-y-4">
              <AlertCircle className="w-12 h-12 mx-auto text-red-500" />
              <h2 className="text-lg font-semibold">Failed to load usage history</h2>
              <Button variant="outline" onClick={() => refetchHistory()}>
                <RefreshCw className="w-4 h-4 mr-2" />
                Retry
              </Button>
            </div>
          ) : totalHistory > 0 ? (
            <div className="space-y-4">
              {pagedHistory.map((period, index) => (
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
                      {/* Show AI tokens in history if there's any non-zero usage OR current plan includes them */}
                      {(period.ai_tokens_used > 0 || usage.usage.ai_tokens.limit > 0) && (
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

              {/* Pagination controls */}
              {totalPages > 1 && (
                <div className="flex items-center justify-between pt-2">
                  <p className="text-sm text-muted-foreground">
                    Showing {historyPage * HISTORY_PAGE_SIZE + 1}–
                    {Math.min((historyPage + 1) * HISTORY_PAGE_SIZE, totalHistory)} of{' '}
                    {totalHistory} periods
                  </p>
                  <div className="flex items-center space-x-2">
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={historyPage === 0}
                      onClick={() => setHistoryPage((p) => p - 1)}
                    >
                      <ChevronLeft className="w-4 h-4" />
                    </Button>
                    <span className="text-sm">
                      {historyPage + 1} / {totalPages}
                    </span>
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={historyPage >= totalPages - 1}
                      onClick={() => setHistoryPage((p) => p + 1)}
                    >
                      <ChevronRight className="w-4 h-4" />
                    </Button>
                  </div>
                </div>
              )}
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

/** Reusable card for displaying a usage metric with optional progress bar */
function UsageCard({
  icon,
  title,
  description,
  metric,
  formatValue,
  remainingLabel,
}: {
  icon: React.ReactNode;
  title: string;
  description: string;
  metric: UsageMetric;
  formatValue: (n: number) => string;
  remainingLabel?: string;
}) {
  const isUnlimited = metric.limit === -1;
  const percentage = getUsagePercentage(metric.used, metric.limit);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center">
          {icon}
          {title}
        </CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div>
          <div className="flex justify-between text-sm mb-2">
            <span>Used</span>
            <span className="font-semibold">
              {formatValue(metric.used)}
              {!isUnlimited && metric.limit > 0 && ` / ${formatValue(metric.limit)}`}
            </span>
          </div>
          {!isUnlimited && metric.limit > 0 && (
            <div className="w-full bg-secondary rounded-full h-3">
              <div
                className={`h-3 rounded-full transition-all ${getUsageColor(percentage)}`}
                style={{ width: `${percentage}%` }}
              />
            </div>
          )}
          <div className="text-xs text-muted-foreground mt-1">
            {isUnlimited
              ? 'Unlimited'
              : metric.limit > 0
                ? `${formatValue(metric.limit - metric.used)} ${remainingLabel ?? ''} remaining`.trim()
                : null}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
