/**
 * Chains API service — chain CRUD, execution, validation, and graph operations.
 */
import type {
  ChainListResponse,
  ChainDefinition,
  ChainCreationResponse,
  ChainExecutionResponse,
  ChainValidationResponse,
  GraphData,
} from '../../types';
import { fetchJson } from './client';

const API_BASE = '/__mockforge/chains';

class ApiService {
  async listChains(): Promise<ChainListResponse> {
    return fetchJson(API_BASE) as Promise<ChainListResponse>;
  }

  async getChain(chainId: string): Promise<ChainDefinition> {
    return fetchJson(`${API_BASE}/${chainId}`) as Promise<ChainDefinition>;
  }

  async getGraph(): Promise<GraphData> {
    const response = await fetchJson('/__mockforge/graph') as { data: GraphData; success: boolean };
    // Handle ApiResponse wrapper
    if (response.success && response.data) {
      return response.data;
    }
    // Fallback: assume response is GraphData directly
    return response as unknown as GraphData;
  }

  async createChain(definition: string): Promise<ChainCreationResponse> {
    return fetchJson(API_BASE, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ definition }),
    }) as Promise<ChainCreationResponse>;
  }

  async updateChain(chainId: string, definition: string): Promise<ChainCreationResponse> {
    return fetchJson(`${API_BASE}/${chainId}`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ definition }),
    }) as Promise<ChainCreationResponse>;
  }

  async deleteChain(chainId: string): Promise<{ message: string }> {
    return fetchJson(`${API_BASE}/${chainId}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  async executeChain(chainId: string, variables?: unknown): Promise<ChainExecutionResponse> {
    return fetchJson(`${API_BASE}/${chainId}/execute`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ variables: variables || {} }),
    }) as Promise<ChainExecutionResponse>;
  }

  async validateChain(chainId: string): Promise<ChainValidationResponse> {
    return fetchJson(`${API_BASE}/${chainId}/validate`, {
      method: 'POST',
    }) as Promise<ChainValidationResponse>;
  }
}

export { ApiService };
