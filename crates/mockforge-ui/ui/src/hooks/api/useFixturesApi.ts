import { logger } from '@/utils/logger';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { fixturesApi, smokeTestsApi, importApi, filesApi } from '../../services/api';
import { queryKeys } from './queryKeys';

/**
 * Files hooks
 */
export function useFileContent() {
  return useMutation({
    mutationFn: ({ path, type }: { path: string; type: string }) =>
      filesApi.getFileContent({ path, type }),
  });
}

export function useSaveFileContent() {
  const _queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ path, content }: { path: string; content: string }) =>
      filesApi.saveFileContent({ path, content }),
    onSuccess: () => {
      // Could invalidate file-related queries here if we had them
    },
  });
}

/**
 * Fixtures hooks
 */
export function useFixtures() {
  return useQuery({
    queryKey: ['fixtures-v2'],
    queryFn: async () => {
      try {
        const fixtures = await fixturesApi.getFixtures();
        return Array.isArray(fixtures) ? fixtures : [];
      } catch (error) {
        logger.error('[FIXTURES ERROR] Failed to fetch fixtures', error);
        throw error;
      }
    },
    retry: false,
    staleTime: 30000,
  });
}

export function useDeleteFixture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: fixturesApi.deleteFixture,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.fixtures });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

export function useDeleteFixturesBulk() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: fixturesApi.deleteFixturesBulk,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.fixtures });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

export function useDownloadFixture() {
  return useMutation({
    mutationFn: fixturesApi.downloadFixture,
  });
}

export function useRenameFixture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ oldPath, newPath }: { oldPath: string; newPath: string }) =>
      fixturesApi.renameFixture(oldPath, newPath),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.fixtures });
    },
  });
}

export function useMoveFixture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ sourcePath, destinationPath }: { sourcePath: string; destinationPath: string }) =>
      fixturesApi.moveFixture(sourcePath, destinationPath),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.fixtures });
    },
  });
}

/**
 * Smoke tests hooks
 */
export function useSmokeTests() {
  return useQuery({
    queryKey: queryKeys.smokeTests,
    queryFn: smokeTestsApi.getSmokeTests,
    staleTime: 10000,
  });
}

export function useRunSmokeTests() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: smokeTestsApi.runSmokeTests,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.smokeTests });
    },
  });
}

/**
 * Import hooks
 */
export function useImportPostman() {
  return useMutation({
    mutationFn: importApi.importPostman,
  });
}

export function useImportInsomnia() {
  return useMutation({
    mutationFn: importApi.importInsomnia,
  });
}

export function useImportCurl() {
  return useMutation({
    mutationFn: importApi.importCurl,
  });
}

export function usePreviewImport() {
  return useMutation({
    mutationFn: importApi.previewImport,
  });
}

export function useImportHistory() {
  return useQuery({
    queryKey: queryKeys.importHistory,
    queryFn: importApi.getImportHistory,
    staleTime: 30000, // Import history doesn't change often
  });
}

export function useClearImportHistory() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: importApi.clearImportHistory,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.importHistory });
    },
  });
}
