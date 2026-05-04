import { useQuery } from '@tanstack/react-query';
import { fetchJson } from '../../services/api/client';
import { isCloudMode } from '../../utils/cloudMode';

export interface CloudActivityEvent {
  id: string;
  timestamp: string;
  event_type: string;
  description: string | null;
  user_id: string | null;
  ip_address: string | null;
}

export function useCloudDashboardActivity(options?: { refetchInterval?: number }) {
  const enabled = isCloudMode();
  return useQuery<CloudActivityEvent[]>({
    queryKey: ['cloudDashboardActivity'],
    queryFn: () => fetchJson('/api/v1/dashboard/logs') as Promise<CloudActivityEvent[]>,
    enabled,
    refetchInterval: options?.refetchInterval ?? 5000,
    staleTime: 4000,
  });
}
