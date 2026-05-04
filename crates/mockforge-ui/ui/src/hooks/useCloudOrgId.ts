/**
 * `useCloudOrgId` — resolves the active org id for cloud-scoped page
 * loads. Reads `default_org_id` from the cached `/api/v1/users/me`
 * response so org-scoped routes (incidents, tunnels, observability,
 * notifications) don't need their own org selector.
 *
 * Returns `null` until the auth response has loaded. Call sites should
 * defer their queries while it's null.
 */
import { useQuery } from '@tanstack/react-query';
import { authApi } from '../services/authApi';
import { isCloudMode } from '../utils/cloudMode';

export function useCloudOrgId(): string | null {
  const { data } = useQuery({
    queryKey: ['auth', 'me'],
    queryFn: () => authApi.getMe(),
    enabled: isCloudMode(),
    staleTime: 60_000,
  });
  return data?.default_org_id ?? null;
}
