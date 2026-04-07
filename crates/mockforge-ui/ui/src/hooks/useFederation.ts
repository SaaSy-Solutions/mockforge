import { useQuery, useMutation, useQueryClient, UseQueryResult } from '@tanstack/react-query';
import { apiErrorMessage } from '@/utils/errorHandling';

const API_BASE = '/api/v1/federation';

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('auth_token');
  return {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
}

export interface Federation {
  id: string;
  name: string;
  description: string;
  org_id: string;
  services: FederationService[];
  created_at: string;
  updated_at: string;
}

export interface FederationService {
  name: string;
  workspace_id: string;
  base_path: string;
  reality_level: 'real' | 'mock_v3' | 'blended' | 'chaos_driven';
  config?: Record<string, unknown>;
  dependencies?: string[];
}

export interface CreateFederationRequest {
  name: string;
  description: string;
  org_id: string;
  services: FederationService[];
}

export interface UpdateFederationRequest {
  name?: string;
  description?: string;
  services?: FederationService[];
}

export interface RouteRequest {
  path: string;
  method: string;
  headers?: Record<string, string>;
  body?: unknown;
}

export interface RouteResponse {
  workspace_id: string;
  service: FederationService;
  service_path: string;
}

// Fetch all federations
export const useFederations = (
  orgId: string
): UseQueryResult<Federation[], Error> => {
  return useQuery<Federation[], Error>({
    queryKey: ['federations', orgId],
    queryFn: async () => {
      const response = await fetch(`${API_BASE}?org_id=${orgId}`, {
        headers: authHeaders(),
      });
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to fetch federations'));
      }
      return response.json();
    },
    enabled: !!orgId,
    refetchInterval: 30000,
  });
};

// Fetch a single federation
export const useFederation = (id: string): UseQueryResult<Federation, Error> => {
  return useQuery<Federation, Error>({
    queryKey: ['federation', id],
    queryFn: async () => {
      const response = await fetch(`${API_BASE}/${id}`, {
        headers: authHeaders(),
      });
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to fetch federation'));
      }
      return response.json();
    },
    enabled: !!id,
  });
};

// Create a new federation
export const useCreateFederation = () => {
  const queryClient = useQueryClient();

  return useMutation<Federation, Error, CreateFederationRequest>({
    mutationFn: async (request) => {
      const response = await fetch(API_BASE, {
        method: 'POST',
        headers: authHeaders(),
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to create federation'));
      }
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['federations'] });
    },
  });
};

// Update a federation
export const useUpdateFederation = () => {
  const queryClient = useQueryClient();

  return useMutation<Federation, Error, { id: string; data: UpdateFederationRequest }>({
    mutationFn: async ({ id, data }) => {
      const response = await fetch(`${API_BASE}/${id}`, {
        method: 'PATCH',
        headers: authHeaders(),
        body: JSON.stringify(data),
      });
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to update federation'));
      }
      return response.json();
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['federations'] });
      queryClient.invalidateQueries({ queryKey: ['federation', variables.id] });
    },
  });
};

// Delete a federation
export const useDeleteFederation = () => {
  const queryClient = useQueryClient();

  return useMutation<void, Error, string>({
    mutationFn: async (id) => {
      const response = await fetch(`${API_BASE}/${id}`, {
        method: 'DELETE',
        headers: authHeaders(),
      });
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to delete federation'));
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['federations'] });
    },
  });
};

// Route a request through federation
export const useRouteRequest = () => {
  return useMutation<RouteResponse, Error, { federationId: string; request: RouteRequest }>({
    mutationFn: async ({ federationId, request }) => {
      const response = await fetch(`${API_BASE}/${federationId}/route`, {
        method: 'POST',
        headers: authHeaders(),
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to route request'));
      }
      return response.json();
    },
  });
};
