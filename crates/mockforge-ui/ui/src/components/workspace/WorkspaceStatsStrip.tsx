import React, { useEffect, useState } from 'react';
import { apiService } from '../../services/api';
import type { WorkspaceStats } from '../../types';
import { Activity, Clock, Route as RouteIcon, Timer } from 'lucide-react';
import { logger } from '@/utils/logger';

interface Props {
  workspaceId: string;
}

const formatNumber = (n: number) => n.toLocaleString();

const formatLatency = (ms: number) => {
  if (!Number.isFinite(ms) || ms <= 0) return '—';
  if (ms < 1) return `${ms.toFixed(2)}ms`;
  if (ms < 1000) return `${ms.toFixed(0)}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
};

const formatTimestamp = (iso?: string) => {
  if (!iso) return 'never';
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  const diffSec = Math.floor((Date.now() - date.getTime()) / 1000);
  if (diffSec < 60) return `${diffSec}s ago`;
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
  return `${Math.floor(diffSec / 86400)}d ago`;
};

const Stat: React.FC<{
  icon: React.ReactNode;
  label: string;
  value: string;
}> = ({ icon, label, value }) => (
  <div className="flex items-center gap-2 px-3 py-2 rounded-md bg-muted/40">
    <span className="text-muted-foreground">{icon}</span>
    <div className="flex flex-col leading-tight">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className="text-sm font-medium">{value}</span>
    </div>
  </div>
);

const WorkspaceStatsStrip: React.FC<Props> = ({ workspaceId }) => {
  const [stats, setStats] = useState<WorkspaceStats | null>(null);

  useEffect(() => {
    let cancelled = false;
    apiService
      .getWorkspaceStats(workspaceId)
      .then((data) => {
        if (!cancelled) setStats(data);
      })
      .catch((err) => {
        logger.error('Failed to load workspace stats', err);
      });
    return () => {
      cancelled = true;
    };
  }, [workspaceId]);

  if (!stats) return null;

  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
      <Stat
        icon={<Activity className="w-4 h-4" />}
        label="Total requests"
        value={formatNumber(stats.total_requests)}
      />
      <Stat
        icon={<RouteIcon className="w-4 h-4" />}
        label="Active routes"
        value={formatNumber(stats.active_routes)}
      />
      <Stat
        icon={<Timer className="w-4 h-4" />}
        label="Avg response"
        value={formatLatency(stats.avg_response_time_ms)}
      />
      <Stat
        icon={<Clock className="w-4 h-4" />}
        label="Last request"
        value={formatTimestamp(stats.last_request_at)}
      />
    </div>
  );
};

export default WorkspaceStatsStrip;
