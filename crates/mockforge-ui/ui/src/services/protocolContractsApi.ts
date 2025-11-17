//! Protocol Contracts API Service
//!
//! This module provides API client functions for managing protocol contracts (gRPC, WebSocket, MQTT, Kafka).

import { authenticatedFetch } from '../utils/apiClient';

// Type definitions matching backend types
export type ProtocolType = 'grpc' | 'websocket' | 'mqtt' | 'kafka';

export interface ProtocolContract {
  contract_id: string;
  version: string;
  protocol: ProtocolType;
  contract: Record<string, unknown>;
}

export interface ListContractsResponse {
  contracts: ProtocolContract[];
  total: number;
}

export interface CreateGrpcContractRequest {
  contract_id: string;
  version: string;
  descriptor_set: string; // base64 encoded
}

export interface CreateWebSocketContractRequest {
  contract_id: string;
  version: string;
  message_types: WebSocketMessageTypeRequest[];
}

export interface WebSocketMessageTypeRequest {
  message_type: string;
  topic?: string;
  schema: Record<string, unknown>;
  direction: 'inbound' | 'outbound' | 'bidirectional';
  description?: string;
  example?: Record<string, unknown>;
}

export interface CreateMqttContractRequest {
  contract_id: string;
  version: string;
  topics: MqttTopicSchemaRequest[];
}

export interface MqttTopicSchemaRequest {
  topic: string;
  qos?: number;
  schema: Record<string, unknown>;
  retained?: boolean;
  description?: string;
  example?: Record<string, unknown>;
}

export interface CreateKafkaContractRequest {
  contract_id: string;
  version: string;
  topics: KafkaTopicSchemaRequest[];
}

export interface KafkaTopicSchemaRequest {
  topic: string;
  key_schema?: TopicSchemaRequest;
  value_schema: TopicSchemaRequest;
  partitions?: number;
  replication_factor?: number;
  description?: string;
  example?: Record<string, unknown>;
  evolution_rules?: EvolutionRulesRequest;
}

export interface TopicSchemaRequest {
  format: 'json' | 'avro' | 'protobuf';
  schema: Record<string, unknown>;
  schema_id?: string;
  version?: string;
}

export interface EvolutionRulesRequest {
  allow_backward_compatible: boolean;
  allow_forward_compatible: boolean;
  require_version_bump: boolean;
}

export interface CompareContractsRequest {
  old_contract_id: string;
  new_contract_id: string;
}

export interface CompareContractsResponse {
  breaking_changes: Array<{
    operation_id: string;
    change_type: string;
    description: string;
  }>;
  non_breaking_changes: Array<{
    operation_id: string;
    change_type: string;
    description: string;
  }>;
  summary: {
    total_operations: number;
    breaking_count: number;
    non_breaking_count: number;
  };
}

export interface ValidateMessageRequest {
  operation_id: string;
  message: Record<string, unknown> | string; // JSON object or base64 encoded binary
  message_format?: 'json' | 'binary';
}

export interface ValidateMessageResponse {
  valid: boolean;
  errors: Array<{
    path: string;
    message: string;
  }>;
  warnings: Array<{
    path: string;
    message: string;
  }>;
}

class ProtocolContractsApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error('Authentication required');
      }
      if (response.status === 403) {
        throw new Error('Access denied');
      }
      const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
      throw new Error((errorData as { error?: string }).error || `HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  /**
   * List all protocol contracts
   * GET /api/v1/contracts?protocol=...
   */
  async listContracts(protocol?: ProtocolType): Promise<ListContractsResponse> {
    const url = protocol
      ? `/api/v1/contracts?protocol=${protocol}`
      : '/api/v1/contracts';
    return this.fetchJson(url) as Promise<ListContractsResponse>;
  }

  /**
   * Get a specific contract
   * GET /api/v1/contracts/{contract_id}
   */
  async getContract(contractId: string): Promise<ProtocolContract> {
    return this.fetchJson(`/api/v1/contracts/${contractId}`) as Promise<ProtocolContract>;
  }

  /**
   * Delete a contract
   * DELETE /api/v1/contracts/{contract_id}
   */
  async deleteContract(contractId: string): Promise<{ message: string }> {
    return this.fetchJson(`/api/v1/contracts/${contractId}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  /**
   * Create a gRPC contract
   * POST /api/v1/contracts/grpc
   */
  async createGrpcContract(request: CreateGrpcContractRequest): Promise<ProtocolContract> {
    return this.fetchJson('/api/v1/contracts/grpc', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ProtocolContract>;
  }

  /**
   * Create a WebSocket contract
   * POST /api/v1/contracts/websocket
   */
  async createWebSocketContract(request: CreateWebSocketContractRequest): Promise<ProtocolContract> {
    return this.fetchJson('/api/v1/contracts/websocket', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ProtocolContract>;
  }

  /**
   * Create an MQTT contract
   * POST /api/v1/contracts/mqtt
   */
  async createMqttContract(request: CreateMqttContractRequest): Promise<ProtocolContract> {
    return this.fetchJson('/api/v1/contracts/mqtt', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ProtocolContract>;
  }

  /**
   * Create a Kafka contract
   * POST /api/v1/contracts/kafka
   */
  async createKafkaContract(request: CreateKafkaContractRequest): Promise<ProtocolContract> {
    return this.fetchJson('/api/v1/contracts/kafka', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ProtocolContract>;
  }

  /**
   * Compare two contracts
   * POST /api/v1/contracts/compare
   */
  async compareContracts(request: CompareContractsRequest): Promise<CompareContractsResponse> {
    return this.fetchJson('/api/v1/contracts/compare', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<CompareContractsResponse>;
  }

  /**
   * Validate a message against a contract
   * POST /api/v1/contracts/{contract_id}/validate
   */
  async validateMessage(
    contractId: string,
    request: ValidateMessageRequest
  ): Promise<ValidateMessageResponse> {
    return this.fetchJson(`/api/v1/contracts/${contractId}/validate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ValidateMessageResponse>;
  }
}

export const protocolContractsApi = new ProtocolContractsApiService();
