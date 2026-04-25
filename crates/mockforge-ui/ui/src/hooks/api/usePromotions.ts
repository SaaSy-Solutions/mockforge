import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  promotionsApi,
  type ApprovePromotionRequest,
  type PromoteScenarioRequest,
  type PromotionStatus,
  type RejectPromotionRequest,
  type ScenarioPromotion,
} from '../../services/api/promotions';

const keys = {
  all: (workspaceId: string) => ['promotions', workspaceId] as const,
  byStatus: (workspaceId: string, status?: PromotionStatus) =>
    ['promotions', workspaceId, status ?? 'all'] as const,
};

export function usePromotions(workspaceId: string | undefined, status?: PromotionStatus) {
  return useQuery<ScenarioPromotion[]>({
    queryKey: keys.byStatus(workspaceId ?? '', status),
    queryFn: () => promotionsApi.list(workspaceId as string, status),
    enabled: !!workspaceId,
    staleTime: 10_000,
  });
}

export function usePromoteScenario(workspaceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      environment,
      request,
    }: {
      environment: string;
      request: PromoteScenarioRequest;
    }) => promotionsApi.promote(workspaceId, environment, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: keys.all(workspaceId) });
    },
  });
}

export function useApprovePromotion(workspaceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      promotionId,
      request,
    }: {
      promotionId: string;
      request: ApprovePromotionRequest;
    }) => promotionsApi.approve(workspaceId, promotionId, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: keys.all(workspaceId) });
    },
  });
}

export function useRejectPromotion(workspaceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      promotionId,
      request,
    }: {
      promotionId: string;
      request: RejectPromotionRequest;
    }) => promotionsApi.reject(workspaceId, promotionId, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: keys.all(workspaceId) });
    },
  });
}
