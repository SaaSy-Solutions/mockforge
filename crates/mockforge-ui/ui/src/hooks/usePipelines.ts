import { useQuery, useMutation, useQueryClient, UseQueryResult } from '@tanstack/react-query';

const API_BASE = '/api/v1/pipelines';

export interface Pipeline {
  id: string;
  name: string;
  definition: PipelineDefinition;
  workspace_id?: string;
  org_id?: string;
  created_at: string;
  updated_at: string;
}

export interface PipelineDefinition {
  enabled: boolean;
  triggers: PipelineTrigger[];
  steps: PipelineStep[];
}

export interface PipelineTrigger {
  event_type: string;
  filters?: Record<string, any>;
}

export interface PipelineStep {
  name: string;
  type: string;
  config: Record<string, any>;
}

export interface CreatePipelineRequest {
  name: string;
  definition: PipelineDefinition;
  workspace_id?: string;
  org_id?: string;
}

export interface UpdatePipelineRequest {
  name?: string;
  definition?: PipelineDefinition;
  enabled?: boolean;
}

export interface PipelineExecution {
  id: string;
  pipeline_id: string;
  trigger_event: Record<string, any>;
  status: 'started' | 'running' | 'completed' | 'failed' | 'cancelled';
  started_at: string;
  completed_at?: string;
  error_message?: string;
  execution_log?: Record<string, any>;
}

export interface ListPipelinesQuery {
  workspace_id?: string;
  org_id?: string;
  enabled?: boolean;
}

export interface ListExecutionsQuery {
  pipeline_id?: string;
  status?: string;
  limit?: number;
  offset?: number;
}

// Fetch all pipelines
export const usePipelines = (
  query?: ListPipelinesQuery
): UseQueryResult<Pipeline[], Error> => {
  const params = new URLSearchParams();
  if (query?.workspace_id) params.append('workspace_id', query.workspace_id);
  if (query?.org_id) params.append('org_id', query.org_id);
  if (query?.enabled !== undefined) params.append('enabled', query.enabled.toString());

  return useQuery<Pipeline[], Error>({
    queryKey: ['pipelines', query],
    queryFn: async () => {
      const response = await fetch(`${API_BASE}?${params.toString()}`);
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to fetch pipelines');
      }
      const data = await response.json();
      return data;
    },
    refetchInterval: 30000, // Refresh every 30 seconds
  });
};

// Fetch a single pipeline
export const usePipeline = (id: string): UseQueryResult<Pipeline, Error> => {
  return useQuery<Pipeline, Error>({
    queryKey: ['pipeline', id],
    queryFn: async () => {
      const response = await fetch(`${API_BASE}/${id}`);
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to fetch pipeline');
      }
      const data = await response.json();
      return data;
    },
  });
};

// Create a new pipeline
export const useCreatePipeline = () => {
  const queryClient = useQueryClient();

  return useMutation<Pipeline, Error, CreatePipelineRequest>({
    mutationFn: async (request) => {
      const response = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to create pipeline');
      }
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipelines'] });
    },
  });
};

// Update a pipeline
export const useUpdatePipeline = () => {
  const queryClient = useQueryClient();

  return useMutation<Pipeline, Error, { id: string; data: UpdatePipelineRequest }>({
    mutationFn: async ({ id, data }) => {
      const response = await fetch(`${API_BASE}/${id}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to update pipeline');
      }
      return response.json();
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['pipelines'] });
      queryClient.invalidateQueries({ queryKey: ['pipeline', variables.id] });
    },
  });
};

// Delete a pipeline
export const useDeletePipeline = () => {
  const queryClient = useQueryClient();

  return useMutation<void, Error, string>({
    mutationFn: async (id) => {
      const response = await fetch(`${API_BASE}/${id}`, {
        method: 'DELETE',
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to delete pipeline');
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pipelines'] });
    },
  });
};

// Trigger a pipeline
export const useTriggerPipeline = () => {
  const queryClient = useQueryClient();

  return useMutation<PipelineExecution, Error, { id: string; event?: Record<string, any> }>({
    mutationFn: async ({ id, event }) => {
      const response = await fetch(`${API_BASE}/${id}/trigger`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(event || {}),
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to trigger pipeline');
      }
      return response.json();
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['pipeline', variables.id] });
      queryClient.invalidateQueries({ queryKey: ['pipeline-executions'] });
    },
  });
};

// Fetch pipeline executions
export const usePipelineExecutions = (
  query?: ListExecutionsQuery
): UseQueryResult<PipelineExecution[], Error> => {
  const params = new URLSearchParams();
  if (query?.pipeline_id) params.append('pipeline_id', query.pipeline_id);
  if (query?.status) params.append('status', query.status);
  if (query?.limit) params.append('limit', query.limit.toString());
  if (query?.offset) params.append('offset', query.offset.toString());

  return useQuery<PipelineExecution[], Error>({
    queryKey: ['pipeline-executions', query],
    queryFn: async () => {
      const response = await fetch(`/api/v1/pipelines/executions?${params.toString()}`);
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to fetch pipeline executions');
      }
      const data = await response.json();
      return data;
    },
    refetchInterval: 10000, // Refresh every 10 seconds for real-time updates
  });
};

// Fetch a single execution
export const usePipelineExecution = (id: string): UseQueryResult<PipelineExecution, Error> => {
  return useQuery<PipelineExecution, Error>({
    queryKey: ['pipeline-execution', id],
    queryFn: async () => {
      const response = await fetch(`/api/v1/pipelines/executions/${id}`);
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to fetch pipeline execution');
      }
      const data = await response.json();
      return data;
    },
    refetchInterval: 5000, // Refresh every 5 seconds for active executions
  });
};
