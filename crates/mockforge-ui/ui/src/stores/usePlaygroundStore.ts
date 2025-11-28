import { logger } from '@/utils/logger';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { apiService } from '../services/api';

/**
 * Playground endpoint information
 */
export interface PlaygroundEndpoint {
  protocol: string;
  method: string;
  path: string;
  description?: string;
  enabled: boolean;
}

/**
 * Request history entry
 */
export interface PlaygroundHistoryEntry {
  id: string;
  protocol: string;
  method: string;
  path: string;
  status_code: number;
  response_time_ms: number;
  timestamp: string;
  request_headers?: Record<string, string>;
  request_body?: unknown;
  graphql_query?: string;
  graphql_variables?: Record<string, unknown>;
}

/**
 * Execute response
 */
export interface ExecuteResponse {
  status_code: number;
  headers: Record<string, string>;
  body: unknown;
  response_time_ms: number;
  request_id: string;
  error?: string;
}

/**
 * REST request configuration
 */
export interface RestRequest {
  method: string;
  path: string;
  headers: Record<string, string>;
  body: string;
  base_url?: string;
}

/**
 * GraphQL request configuration
 */
export interface GraphQLRequest {
  query: string;
  variables: Record<string, unknown>;
  operation_name?: string;
  base_url?: string;
}

interface PlaygroundState {
  // Protocol selection
  protocol: 'rest' | 'graphql';

  // Endpoints
  endpoints: PlaygroundEndpoint[];
  endpointsLoading: boolean;
  endpointsError: string | null;

  // Current request
  restRequest: RestRequest;
  graphQLRequest: GraphQLRequest;

  // Current response
  currentResponse: ExecuteResponse | null;
  mockAIResponse: ExecuteResponse | null; // Separate storage for MockAI preview
  responseLoading: boolean;
  responseError: string | null;

  // History
  history: PlaygroundHistoryEntry[];
  historyLoading: boolean;
  historyError: string | null;

  // GraphQL introspection
  graphQLSchema: {
    schema: unknown;
    query_types: string[];
    mutation_types: string[];
    subscription_types: string[];
  } | null;
  introspectionLoading: boolean;
  introspectionError: string | null;
}

interface PlaygroundActions {
  // Protocol
  setProtocol: (protocol: 'rest' | 'graphql') => void;

  // Endpoints
  loadEndpoints: () => Promise<void>;

  // REST request
  setRestRequest: (request: Partial<RestRequest>) => void;
  executeRestRequest: (useMockAI?: boolean) => Promise<void>;

  // GraphQL request
  setGraphQLRequest: (request: Partial<GraphQLRequest>) => void;
  executeGraphQLRequest: () => Promise<void>;

  // Response
  clearResponse: () => void;

  // History
  loadHistory: (params?: { limit?: number; protocol?: string }) => Promise<void>;
  replayRequest: (requestId: string) => Promise<void>;
  clearHistory: () => void;

  // GraphQL introspection
  loadGraphQLIntrospection: () => Promise<void>;
}

export const usePlaygroundStore = create<PlaygroundState & PlaygroundActions>()(
  persist(
    (set, get) => ({
      // Initial state
      protocol: 'rest',
      endpoints: [],
      endpointsLoading: false,
      endpointsError: null,
      restRequest: {
        method: 'GET',
        path: '',
        headers: {},
        body: '',
      },
      graphQLRequest: {
        query: '',
        variables: {},
        operation_name: undefined,
      },
      currentResponse: null,
      mockAIResponse: null,
      responseLoading: false,
      responseError: null,
      history: [],
      historyLoading: false,
      historyError: null,
      graphQLSchema: null,
      introspectionLoading: false,
      introspectionError: null,

      // Protocol
      setProtocol: (protocol) => {
        set({ protocol });
      },

      // Endpoints
      loadEndpoints: async () => {
        set({ endpointsLoading: true, endpointsError: null });
        try {
          // Get active workspace to filter endpoints
          const activeWorkspace = useWorkspaceStore.getState().activeWorkspace;
          const workspaceId = activeWorkspace?.id;

          const endpoints = await apiService.listPlaygroundEndpoints(workspaceId);

          set({ endpoints, endpointsLoading: false });
        } catch (error) {
          logger.error('Failed to load playground endpoints', error);
          set({
            endpointsError: error instanceof Error ? error.message : 'Failed to load endpoints',
            endpointsLoading: false,
            endpoints: [],
          });
        }
      },

      // REST request
      setRestRequest: (request) => {
        set((state) => ({
          restRequest: { ...state.restRequest, ...request },
        }));
      },

      executeRestRequest: async (useMockAI = false) => {
        const { restRequest } = get();
        set({ responseLoading: true, responseError: null });
        try {
          // Get active workspace ID
          const activeWorkspace = useWorkspaceStore.getState().activeWorkspace;
          const workspaceId = activeWorkspace?.id;

          const response = await apiService.executeRestRequest({
            method: restRequest.method,
            path: restRequest.path,
            headers: Object.keys(restRequest.headers).length > 0 ? restRequest.headers : undefined,
            body: restRequest.body ? JSON.parse(restRequest.body) : undefined,
            base_url: restRequest.base_url,
            use_mockai: useMockAI,
            workspace_id: workspaceId,
          });

          // Store response in appropriate location
          if (useMockAI) {
            set({ mockAIResponse: response, responseLoading: false });
          } else {
            set({ currentResponse: response, responseLoading: false });
          }

          // Reload history to include the new request
          await get().loadHistory();
        } catch (error) {
          logger.error('Failed to execute REST request', error);
          set({
            responseError: error instanceof Error ? error.message : 'Failed to execute request',
            responseLoading: false,
          });
        }
      },

      // GraphQL request
      setGraphQLRequest: (request) => {
        set((state) => ({
          graphQLRequest: { ...state.graphQLRequest, ...request },
        }));
      },

      executeGraphQLRequest: async () => {
        const { graphQLRequest } = get();
        set({ responseLoading: true, responseError: null });
        try {
          // Get active workspace ID
          const activeWorkspace = useWorkspaceStore.getState().activeWorkspace;
          const workspaceId = activeWorkspace?.id;

          const response = await apiService.executeGraphQLQuery({
            query: graphQLRequest.query,
            variables: Object.keys(graphQLRequest.variables).length > 0 ? graphQLRequest.variables : undefined,
            operation_name: graphQLRequest.operation_name,
            base_url: graphQLRequest.base_url,
            workspace_id: workspaceId,
          });
          set({ currentResponse: response, responseLoading: false });

          // Reload history to include the new request
          await get().loadHistory();
        } catch (error) {
          logger.error('Failed to execute GraphQL request', error);
          set({
            responseError: error instanceof Error ? error.message : 'Failed to execute request',
            responseLoading: false,
          });
        }
      },

      // Response
      clearResponse: () => {
        set({ currentResponse: null, responseError: null });
      },

      // History
      loadHistory: async (params) => {
        set({ historyLoading: true, historyError: null });
        try {
          // Get active workspace ID for filtering
          const activeWorkspace = useWorkspaceStore.getState().activeWorkspace;
          const workspaceId = activeWorkspace?.id;

          const historyParams = {
            ...params,
            workspace_id: workspaceId,
          };

          const history = await apiService.getPlaygroundHistory(historyParams);

          set({ history, historyLoading: false });
        } catch (error) {
          logger.error('Failed to load playground history', error);
          set({
            historyError: error instanceof Error ? error.message : 'Failed to load history',
            historyLoading: false,
            history: [],
          });
        }
      },

      replayRequest: async (requestId) => {
        set({ responseLoading: true, responseError: null });
        try {
          const response = await apiService.replayRequest(requestId);
          set({ currentResponse: response, responseLoading: false });

          // Reload history
          await get().loadHistory();
        } catch (error) {
          logger.error('Failed to replay request', error);
          set({
            responseError: error instanceof Error ? error.message : 'Failed to replay request',
            responseLoading: false,
          });
        }
      },

      clearHistory: () => {
        set({ history: [] });
      },

      // GraphQL introspection
      loadGraphQLIntrospection: async () => {
        set({ introspectionLoading: true, introspectionError: null });
        try {
          const schema = await apiService.graphQLIntrospect();
          set({ graphQLSchema: schema, introspectionLoading: false });
        } catch (error) {
          logger.error('Failed to load GraphQL introspection', error);
          set({
            introspectionError: error instanceof Error ? error.message : 'Failed to load introspection',
            introspectionLoading: false,
          });
        }
      },
    }),
    {
      name: 'mockforge-playground',
      partialize: (state) => {
        // Persist playground state with workspace context
        // Note: Workspace-specific persistence can be enhanced when needed
        return {
          protocol: state.protocol,
          restRequest: state.restRequest,
          graphQLRequest: state.graphQLRequest,
          // Don't persist responses, history, or loading states
        };
      },
    }
  )
);
