import { useQuery, useMutation, useQueryClient, type UseQueryResult } from '@tanstack/react-query';
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

// -----------------------------------------------------------------------------
// Federation-wide scenario activations
// -----------------------------------------------------------------------------

export interface ServiceScenarioOverride {
  reality_level?: 'real' | 'mock_v3' | 'blended' | 'chaos_driven';
  chaos_level?: number;
  failure_rate?: number;
  latency_ms?: number;
  notes?: string;
  metadata?: Record<string, unknown>;
}

export interface PerServiceActivationState {
  service_name: string;
  workspace_id: string;
  status: 'pending' | 'applied' | 'failed';
  error?: string | null;
  last_observed_at?: string | null;
}

export interface FederationScenarioActivation {
  id: string;
  federation_id: string;
  scenario_id?: string | null;
  scenario_name: string;
  manifest_snapshot: unknown;
  service_overrides: Record<string, ServiceScenarioOverride>;
  status: 'active' | 'deactivated' | 'failed';
  per_service_state: PerServiceActivationState[];
  activated_by: string;
  activated_at: string;
  deactivated_at?: string | null;
}

export interface ActivateScenarioRequest {
  scenario_id?: string;
  scenario_name?: string;
  manifest: unknown;
  service_overrides?: Record<string, ServiceScenarioOverride>;
}

// Fetch the active scenario (if any) for a federation
export const useActiveFederationScenario = (
  federationId: string
): UseQueryResult<FederationScenarioActivation | null, Error> => {
  return useQuery<FederationScenarioActivation | null, Error>({
    queryKey: ['federation-active-scenario', federationId],
    queryFn: async () => {
      const response = await fetch(`${API_BASE}/${federationId}/scenarios/active`, {
        headers: authHeaders(),
      });
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(
          apiErrorMessage(response, errorData, 'Failed to fetch active scenario')
        );
      }
      return response.json();
    },
    enabled: !!federationId,
    refetchInterval: 10000,
  });
};

// Activate a scenario on a federation
export const useActivateFederationScenario = () => {
  const queryClient = useQueryClient();

  return useMutation<
    FederationScenarioActivation,
    Error,
    { federationId: string; request: ActivateScenarioRequest }
  >({
    mutationFn: async ({ federationId, request }) => {
      const response = await fetch(`${API_BASE}/${federationId}/scenarios/activate`, {
        method: 'POST',
        headers: authHeaders(),
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to activate scenario'));
      }
      return response.json();
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: ['federation-active-scenario', variables.federationId],
      });
    },
  });
};

// -----------------------------------------------------------------------------
// Org-scoped scenarios (for the activation picker)
// -----------------------------------------------------------------------------

export interface OrgScenarioEntry {
  id: string;
  name: string;
  slug: string;
  description: string;
  current_version: string;
  category: string;
  tags: string[];
  manifest_json: unknown;
  created_at: string;
  updated_at: string;
}

// List scenarios owned by the caller's org — powers the picker dropdown.
export const useOrgScenarios = (): UseQueryResult<OrgScenarioEntry[], Error> => {
  return useQuery<OrgScenarioEntry[], Error>({
    queryKey: ['org-scenarios'],
    queryFn: async () => {
      const response = await fetch(`/api/v1/scenarios`, { headers: authHeaders() });
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to fetch scenarios'));
      }
      return response.json();
    },
  });
};

// Deactivate the active scenario on a federation
export const useDeactivateFederationScenario = () => {
  const queryClient = useQueryClient();

  return useMutation<FederationScenarioActivation, Error, { federationId: string }>({
    mutationFn: async ({ federationId }) => {
      const response = await fetch(`${API_BASE}/${federationId}/scenarios/active`, {
        method: 'DELETE',
        headers: authHeaders(),
      });
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to deactivate scenario'));
      }
      return response.json();
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: ['federation-active-scenario', variables.federationId],
      });
    },
  });
};
