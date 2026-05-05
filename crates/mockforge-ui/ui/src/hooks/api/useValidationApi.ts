import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { validationApi } from '../../services/api';
import { queryKeys } from './queryKeys';

/**
 * Validation hooks
 */
export function useValidation(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: queryKeys.validation,
    queryFn: validationApi.getValidation,
    staleTime: 30000,
    enabled: options?.enabled ?? true,
  });
}

export function useUpdateValidation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: validationApi.updateValidation,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.validation });
    },
  });
}
