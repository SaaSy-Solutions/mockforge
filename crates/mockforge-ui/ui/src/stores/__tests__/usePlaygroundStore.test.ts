import { describe, it, expect, beforeEach, vi } from 'vitest';
import { usePlaygroundStore } from '../usePlaygroundStore';
import { apiService } from '../../services/api';

// Mock the API service
vi.mock('../../services/api', () => ({
  apiService: {
    listPlaygroundEndpoints: vi.fn(),
    executeRestRequest: vi.fn(),
    executeGraphQLQuery: vi.fn(),
    getPlaygroundHistory: vi.fn(),
    replayRequest: vi.fn(),
    graphQLIntrospect: vi.fn(),
  },
}));

describe('usePlaygroundStore', () => {
  beforeEach(() => {
    // Reset store state
    usePlaygroundStore.setState({
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
      },
      currentResponse: null,
      responseLoading: false,
      responseError: null,
      history: [],
      historyLoading: false,
      historyError: null,
      graphQLSchema: null,
      introspectionLoading: false,
      introspectionError: null,
    });
  });

  it('sets protocol', () => {
    usePlaygroundStore.getState().setProtocol('graphql');

    expect(usePlaygroundStore.getState().protocol).toBe('graphql');
  });

  it('sets REST request', () => {
    usePlaygroundStore.getState().setRestRequest({
      method: 'POST',
      path: '/api/users',
    });

    const restRequest = usePlaygroundStore.getState().restRequest;
    expect(restRequest.method).toBe('POST');
    expect(restRequest.path).toBe('/api/users');
  });

  it('sets GraphQL request', () => {
    usePlaygroundStore.getState().setGraphQLRequest({
      query: 'query { user { name } }',
    });

    const graphQLRequest = usePlaygroundStore.getState().graphQLRequest;
    expect(graphQLRequest.query).toBe('query { user { name } }');
  });

  it('loads endpoints', async () => {
    const mockEndpoints = [
      {
        protocol: 'rest',
        method: 'GET',
        path: '/api/users',
        enabled: true,
      },
    ];

    (apiService.listPlaygroundEndpoints as any).mockResolvedValue(mockEndpoints);

    await usePlaygroundStore.getState().loadEndpoints();

    expect(usePlaygroundStore.getState().endpoints).toEqual(mockEndpoints);
    expect(usePlaygroundStore.getState().endpointsLoading).toBe(false);
  });

  it('clears response', () => {
    usePlaygroundStore.setState({
      currentResponse: {
        status_code: 200,
        headers: {},
        body: {},
        response_time_ms: 150,
        request_id: 'test',
      },
    });

    usePlaygroundStore.getState().clearResponse();

    expect(usePlaygroundStore.getState().currentResponse).toBeNull();
  });
});
