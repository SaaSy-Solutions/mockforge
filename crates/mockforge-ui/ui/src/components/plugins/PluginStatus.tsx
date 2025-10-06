import { logger } from '@/utils/logger';
import React, { useState, useEffect } from 'react';
import {
  Activity,
  CheckCircle,
  XCircle,
  AlertTriangle,
  TrendingUp,
  Clock,
  RefreshCw
} from 'lucide-react';
import {
  Card,
  Button,
  Alert,
  Progress
} from '../ui/DesignSystem';

interface PluginStats {
  total_plugins: number;
  discovered: number;
  loaded: number;
  failed: number;
  skipped: number;
  success_rate: number;
}

interface PluginHealth {
  id: string;
  healthy: boolean;
  message: string;
  last_check: string;
}

interface PluginStatusData {
  stats: PluginStats;
  health: PluginHealth[];
  last_updated?: string;
}

export function PluginStatus() {
  const [status, setStatus] = useState<PluginStatusData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchStatus();
  }, []);

  const fetchStatus = async () => {
    try {
      setLoading(true);
      const response = await fetch('/__mockforge/plugins/status');
      const data = await response.json();

      if (data.success) {
        setStatus(data.data);
      } else {
        setError(data.error);
      }
    } catch {
      setError('Failed to fetch plugin status');
    } finally {
      setLoading(false);
    }
  };

  const getHealthIcon = (healthy: boolean) => {
    return healthy ? (
      <CheckCircle className="w-4 h-4 text-green-500" />
    ) : (
      <XCircle className="w-4 h-4 text-red-500" />
    );
  };

  const formatDateTime = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

  if (loading) {
    return (
      <Card>
        <div className="p-6">
          <div className="animate-pulse space-y-4">
            <div className="h-8 bg-gray-200 rounded w-1/4"></div>
            <div className="h-4 bg-gray-200 rounded w-1/2"></div>
            <div className="h-32 bg-gray-200 rounded"></div>
          </div>
        </div>
      </Card>
    );
  }

  if (error || !status) {
    return (
      <Alert variant="destructive">
        <AlertTriangle className="h-4 w-4" />
        <div>
          <strong>Error loading plugin status:</strong> {error || 'Unknown error'}
        </div>
      </Alert>
    );
  }

  const { stats, health } = status;
  const healthyPlugins = health.filter(h => h.healthy).length;
  const unhealthyPlugins = health.filter(h => !h.healthy).length;

  return (
    <div className="space-y-6">
      {/* Overall Statistics */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <div className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-600">Total Plugins</p>
                <p className="text-2xl font-bold">{stats.total_plugins}</p>
              </div>
              <Activity className="w-8 h-8 text-blue-500" />
            </div>
          </div>
        </Card>

        <Card>
          <div className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-600">Loaded</p>
                <p className="text-2xl font-bold text-green-600">{stats.loaded}</p>
              </div>
              <CheckCircle className="w-8 h-8 text-green-500" />
            </div>
          </div>
        </Card>

        <Card>
          <div className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-600">Failed</p>
                <p className="text-2xl font-bold text-red-600">{stats.failed}</p>
              </div>
              <XCircle className="w-8 h-8 text-red-500" />
            </div>
          </div>
        </Card>

        <Card>
          <div className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-600">Success Rate</p>
                <p className="text-2xl font-bold">{stats.success_rate.toFixed(1)}%</p>
              </div>
              <TrendingUp className="w-8 h-8 text-green-500" />
            </div>
          </div>
        </Card>
      </div>

      {/* Success Rate Progress */}
      <Card>
        <div className="p-6">
          <h3 className="text-lg font-medium mb-4">Plugin Loading Success Rate</h3>
          <div className="space-y-2">
            <div className="flex justify-between text-sm">
              <span>Success Rate</span>
              <span>{stats.success_rate.toFixed(1)}%</span>
            </div>
            <Progress value={stats.success_rate} className="w-full" />
            <div className="flex justify-between text-xs text-gray-500">
              <span>Loaded: {stats.loaded}</span>
              <span>Failed: {stats.failed}</span>
              <span>Skipped: {stats.skipped}</span>
            </div>
          </div>
        </div>
      </Card>

      {/* Health Status */}
      <Card>
        <div className="p-6">
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-lg font-medium">Plugin Health Status</h3>
            <Button variant="outline" size="sm" onClick={fetchStatus}>
              <RefreshCw className="w-4 h-4" />
            </Button>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
            <div className="flex items-center gap-2">
              <CheckCircle className="w-5 h-5 text-green-500" />
              <span className="text-sm">
                <strong>{healthyPlugins}</strong> healthy
              </span>
            </div>
            <div className="flex items-center gap-2">
              <XCircle className="w-5 h-5 text-red-500" />
              <span className="text-sm">
                <strong>{unhealthyPlugins}</strong> unhealthy
              </span>
            </div>
          </div>

          <div className="space-y-2 max-h-64 overflow-y-auto">
            {health.map((plugin) => (
              <div
                key={plugin.id}
                className={`flex items-center justify-between p-3 rounded-lg border ${
                  plugin.healthy
                    ? 'border-green-200 bg-green-50'
                    : 'border-red-200 bg-red-50'
                }`}
              >
                <div className="flex items-center gap-3">
                  {getHealthIcon(plugin.healthy)}
                  <div>
                    <div className="font-medium">{plugin.id}</div>
                    <div className="text-sm text-gray-600">{plugin.message}</div>
                  </div>
                </div>
                <div className="text-xs text-gray-500">
                  <Clock className="w-3 h-3 inline mr-1" />
                  {formatDateTime(plugin.last_check)}
                </div>
              </div>
            ))}
          </div>

          {status.last_updated && (
            <div className="mt-4 text-xs text-gray-500 text-center">
              Last updated: {formatDateTime(status.last_updated)}
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
