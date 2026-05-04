import { useMemo, useState } from 'react';
import { Clock, FileText, Globe, User } from 'lucide-react';
import { Card } from '../ui/Card';
import { Input } from '../ui/input';
import { Badge } from '../ui/DesignSystem';
import { ResponsiveTable, type ResponsiveTableColumn } from '../ui/ResponsiveTable';
import { SkeletonTable } from '../ui/Skeleton';
import { useCloudDashboardActivity } from '../../hooks/useApi';

const fmtTime = (iso: string) =>
  new Date(iso).toLocaleString([], {
    month: 'short',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });

function eventBadge(eventType: string): 'success' | 'warning' | 'error' | 'info' {
  if (eventType.endsWith('_deleted') || eventType.endsWith('_revoked')) return 'error';
  if (eventType.endsWith('_failed') || eventType.includes('error')) return 'error';
  if (eventType.endsWith('_created') || eventType.endsWith('_added')) return 'success';
  if (eventType.endsWith('_updated') || eventType.endsWith('_changed')) return 'warning';
  return 'info';
}

const Title = () => (
  <div className="flex items-center gap-2">
    <FileText className="h-5 w-5" />
    <span>Recent Activity</span>
    <span className="text-xs text-gray-500 ml-1">(Auto-refreshes every 5s)</span>
  </div>
);

export function CloudActivityFeed() {
  const [search, setSearch] = useState('');
  const { data, isLoading } = useCloudDashboardActivity();

  const rows = useMemo(() => {
    if (!data) return [];
    if (!search) return data;
    const q = search.toLowerCase();
    return data.filter(
      (e) =>
        e.event_type.toLowerCase().includes(q) ||
        (e.description ?? '').toLowerCase().includes(q) ||
        (e.ip_address ?? '').toLowerCase().includes(q),
    );
  }, [data, search]);

  const columns: ResponsiveTableColumn[] = [
    {
      key: 'event_type',
      label: 'Event',
      priority: 'high',
      render: (value: unknown) => (
        <Badge variant={eventBadge(value as string)}>{value as string}</Badge>
      ),
      width: '180px',
    },
    {
      key: 'description',
      label: 'Description',
      priority: 'high',
      render: (value: unknown) => (
        <span className="text-sm text-gray-900 dark:text-gray-100 truncate" title={(value as string) ?? ''}>
          {(value as string) ?? '—'}
        </span>
      ),
    },
    {
      key: 'timestamp',
      label: 'Time',
      priority: 'medium',
      render: (value: unknown) => (
        <div className="flex items-center gap-1">
          <Clock className="h-3 w-3 text-gray-600 dark:text-gray-400" />
          <span className="font-mono text-sm">{fmtTime(value as string)}</span>
        </div>
      ),
      width: '170px',
    },
    {
      key: 'user_id',
      label: 'User',
      priority: 'low',
      hideOnMobile: true,
      render: (value: unknown) => (
        <div className="flex items-center gap-1">
          <User className="h-3 w-3 text-gray-600 dark:text-gray-400" />
          <span className="font-mono text-xs">{value ? String(value).slice(0, 8) : '—'}</span>
        </div>
      ),
      width: '110px',
    },
    {
      key: 'ip_address',
      label: 'IP',
      priority: 'low',
      hideOnMobile: true,
      render: (value: unknown) => (
        <div className="flex items-center gap-1">
          <Globe className="h-3 w-3 text-gray-600 dark:text-gray-400" />
          <span className="font-mono text-xs">{(value as string) ?? '—'}</span>
        </div>
      ),
      width: '140px',
    },
  ];

  if (isLoading) {
    return (
      <Card title={<Title />}>
        <SkeletonTable rows={5} cols={5} />
      </Card>
    );
  }

  return (
    <Card title={<Title />}>
      <div className="mb-4">
        <Input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search event, description, IP…"
          className="max-w-sm"
        />
      </div>
      <div className="max-h-80 overflow-y-auto">
        <ResponsiveTable
          columns={columns}
          data={rows}
          stackOnMobile={true}
          sortable={true}
          emptyMessage="No recent activity"
          className="animate-fade-in-up"
        />
      </div>
    </Card>
  );
}
