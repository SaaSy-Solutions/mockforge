import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { realityApi, consistencyApi } from '../../services/api';
import { queryKeys } from './queryKeys';

/**
 * Reality Slider hooks
 */
export function useRealityLevel() {
  return useQuery({
    queryKey: queryKeys.realityLevel,
    queryFn: () => realityApi.getRealityLevel(),
    staleTime: 10000, // Consider data stale after 10 seconds
    refetchInterval: 30000, // Refetch every 30 seconds
  });
}

export function useSetRealityLevel() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (level: number) => realityApi.setRealityLevel(level),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.realityLevel });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

export function useRealityPresets() {
  return useQuery({
    queryKey: queryKeys.realityPresets,
    queryFn: () => realityApi.listPresets(),
    staleTime: 60000, // Presets don't change often
  });
}

export function useImportRealityPreset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (path: string) => realityApi.importPreset(path),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.realityLevel });
      queryClient.invalidateQueries({ queryKey: queryKeys.realityPresets });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

export function useExportRealityPreset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ name, description }: { name: string; description?: string }) =>
      realityApi.exportPreset(name, description),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.realityPresets });
    },
  });
}

/**
 * Lifecycle preset hooks
 */
export function useLifecyclePresets() {
  return useQuery({
    queryKey: queryKeys.lifecyclePresets,
    queryFn: () => consistencyApi.listLifecyclePresets(),
    staleTime: 60000, // Presets don't change often
  });
}

export function useLifecyclePresetDetails(presetName: string) {
  return useQuery({
    queryKey: queryKeys.lifecyclePreset(presetName),
    queryFn: () => consistencyApi.getLifecyclePresetDetails(presetName),
    enabled: !!presetName,
    staleTime: 60000,
  });
}

export function useApplyLifecyclePreset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ workspace, personaId, preset }: { workspace: string; personaId: string; preset: string }) =>
      consistencyApi.applyLifecyclePreset(workspace, personaId, preset),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['consistency', 'state'] });
      queryClient.invalidateQueries({ queryKey: ['consistency', 'persona'] });
      queryClient.invalidateQueries({ queryKey: queryKeys.lifecyclePresets });
    },
  });
}
