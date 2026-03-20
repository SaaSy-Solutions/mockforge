/**
 * Consistency API service — lifecycle presets and entity management.
 */
import { fetchJsonWithErrorBody } from './client';

class ConsistencyApiService {
  /**
   * List all available lifecycle presets
   */
  async listLifecyclePresets(): Promise<{
    presets: Array<{
      name: string;
      id: string;
      description: string;
    }>;
  }> {
    return fetchJsonWithErrorBody('/api/v1/consistency/lifecycle-presets') as Promise<{
      presets: Array<{
        name: string;
        id: string;
        description: string;
      }>;
    }>;
  }

  /**
   * Get details of a specific lifecycle preset
   */
  async getLifecyclePresetDetails(presetName: string): Promise<{
    preset: {
      name: string;
      id: string;
      description: string;
    };
    initial_state: string;
    states: Array<{
      from: string;
      to: string;
      after_days: number | null;
      condition: string | null;
    }>;
    affected_endpoints: string[];
  }> {
    return fetchJsonWithErrorBody(`/api/v1/consistency/lifecycle-presets/${encodeURIComponent(presetName)}`) as Promise<{
      preset: {
        name: string;
        id: string;
        description: string;
      };
      initial_state: string;
      states: Array<{
        from: string;
        to: string;
        after_days: number | null;
        condition: string | null;
      }>;
      affected_endpoints: string[];
    }>;
  }

  /**
   * Apply a lifecycle preset to a persona
   */
  async applyLifecyclePreset(
    workspace: string,
    personaId: string,
    preset: string
  ): Promise<{
    success: boolean;
    workspace: string;
    persona_id: string;
    preset: string;
    lifecycle_state: string;
  }> {
    return fetchJsonWithErrorBody(`/api/v1/consistency/lifecycle-presets/apply?workspace=${encodeURIComponent(workspace)}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ persona_id: personaId, preset }),
    }) as Promise<{
      success: boolean;
      workspace: string;
      persona_id: string;
      preset: string;
      lifecycle_state: string;
    }>;
  }

  /**
   * List all entities for a workspace
   */
  async listEntities(workspace = 'default'): Promise<{
    workspace: string;
    entities: Array<{
      entity_type: string;
      entity_id: string;
      data: Record<string, unknown>;
      seen_in_protocols: string[];
      created_at: string;
      updated_at: string;
      persona_id: string | null;
    }>;
    count: number;
  }> {
    return fetchJsonWithErrorBody(`/api/v1/consistency/entities?workspace=${encodeURIComponent(workspace)}`) as Promise<{
      workspace: string;
      entities: Array<{
        entity_type: string;
        entity_id: string;
        data: Record<string, unknown>;
        seen_in_protocols: string[];
        created_at: string;
        updated_at: string;
        persona_id: string | null;
      }>;
      count: number;
    }>;
  }

  /**
   * Get a specific entity by type and ID
   */
  async getEntity(entityType: string, entityId: string, workspace = 'default'): Promise<{
    entity_type: string;
    entity_id: string;
    data: Record<string, unknown>;
    seen_in_protocols: string[];
    created_at: string;
    updated_at: string;
    persona_id: string | null;
  }> {
    return fetchJsonWithErrorBody(
      `/api/v1/consistency/entities/${encodeURIComponent(entityType)}/${encodeURIComponent(entityId)}?workspace=${encodeURIComponent(workspace)}`
    ) as Promise<{
      entity_type: string;
      entity_id: string;
      data: Record<string, unknown>;
      seen_in_protocols: string[];
      created_at: string;
      updated_at: string;
      persona_id: string | null;
    }>;
  }
}

export { ConsistencyApiService };
