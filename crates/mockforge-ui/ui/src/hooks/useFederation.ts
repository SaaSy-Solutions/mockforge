import { useQuery, useMutation, useQueryClient, UseQueryResult } from '@tanstack/react-query';

const API_BASE = '/api/v1/federation';

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
  config?: Record<string, any>;
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
  body?: any;
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
      const response = await fetch(`${API_BASE}?org_id=${orgId}`);
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to fetch federations');
      }
      const data = await response.json();
      return data;
    },
    refetchInterval: 30000, // Refresh every 30 seconds
  });
};

// Fetch a single federation
export const useFederation = (id: string): UseQueryResult<Federation, Error> => {
  return useQuery<Federation, Error>({
    queryKey: ['federation', id],
    queryFn: async () => {
      const response = await fetch(`${API_BASE}/${id}`);
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to fetch federation');
      }
      const data = await response.json();
      return data;
    },
  });
};

// Create a new federation
export const useCreateFederation = () => {
  const queryClient = useQueryClient();

  return useMutation<Federation, Error, CreateFederationRequest>({
    mutationFn: async (request) => {
      const response = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to create federation');
      }
      return response.json();
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['federations', variables.org_id] });
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
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to update federation');
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
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to delete federation');
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
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to route request');
      }
      return response.json();
    },
  });
};
