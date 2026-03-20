import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { proxyApi, type ProxyRuleRequest } from '../../services/api';
import { queryKeys } from './queryKeys';

/**
 * Proxy replacement rules hooks
 */
export function useProxyRules() {
  return useQuery({
    queryKey: queryKeys.proxyRules,
    queryFn: () => proxyApi.getProxyRules(),
    staleTime: 10000, // Cache for 10 seconds
    refetchInterval: 5000, // Auto-refresh every 5 seconds
  });
}

export function useProxyRule(id: number) {
  return useQuery({
    queryKey: [...queryKeys.proxyRules, id],
    queryFn: () => proxyApi.getProxyRule(id),
    enabled: id !== undefined && id !== null,
    staleTime: 10000,
  });
}

export function useCreateProxyRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (rule: ProxyRuleRequest) => proxyApi.createProxyRule(rule),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.proxyRules });
    },
  });
}

export function useUpdateProxyRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, rule }: { id: number; rule: ProxyRuleRequest }) =>
      proxyApi.updateProxyRule(id, rule),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.proxyRules });
      queryClient.invalidateQueries({ queryKey: [...queryKeys.proxyRules, variables.id] });
    },
  });
}

export function useDeleteProxyRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: number) => proxyApi.deleteProxyRule(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.proxyRules });
    },
  });
}

export function useProxyInspect(limit?: number) {
  return useQuery({
    queryKey: [...queryKeys.proxyInspect, limit],
    queryFn: () => proxyApi.getProxyInspect(limit),
    staleTime: 2000, // Very short cache for real-time inspection
    refetchInterval: 2000, // Auto-refresh every 2 seconds
  });
}
