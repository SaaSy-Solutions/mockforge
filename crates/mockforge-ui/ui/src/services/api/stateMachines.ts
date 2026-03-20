/**
 * State Machine API service — CRUD and instance management for state machines.
 */
import { fetchJson } from './client';

class StateMachineApiMixin {
  async getStateMachines(): Promise<{ state_machines: Array<{ resource_type: string; state_count: number; transition_count: number; sub_scenario_count: number; has_visual_layout: boolean }>; total: number }> {
    return fetchJson('/__mockforge/api/state-machines') as Promise<{ state_machines: Array<{ resource_type: string; state_count: number; transition_count: number; sub_scenario_count: number; has_visual_layout: boolean }>; total: number }>;
  }

  async getStateMachine(resourceType: string): Promise<{ state_machine: unknown; visual_layout?: unknown }> {
    return fetchJson(`/__mockforge/api/state-machines/${encodeURIComponent(resourceType)}`) as Promise<{ state_machine: unknown; visual_layout?: unknown }>;
  }

  async createStateMachine(stateMachine: unknown, visualLayout?: unknown): Promise<{ state_machine: unknown; visual_layout?: unknown }> {
    return fetchJson('/__mockforge/api/state-machines', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ state_machine: stateMachine, visual_layout: visualLayout }),
    }) as Promise<{ state_machine: unknown; visual_layout?: unknown }>;
  }

  async updateStateMachine(resourceType: string, stateMachine: unknown, visualLayout?: unknown): Promise<{ state_machine: unknown; visual_layout?: unknown }> {
    return fetchJson(`/__mockforge/api/state-machines/${encodeURIComponent(resourceType)}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ state_machine: stateMachine, visual_layout: visualLayout }),
    }) as Promise<{ state_machine: unknown; visual_layout?: unknown }>;
  }

  async deleteStateMachine(resourceType: string): Promise<void> {
    await fetchJson(`/__mockforge/api/state-machines/${encodeURIComponent(resourceType)}`, {
      method: 'DELETE',
    });
  }

  async getStateInstances(): Promise<{ instances: Array<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }>; total: number }> {
    return fetchJson('/__mockforge/api/state-machines/instances') as Promise<{ instances: Array<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }>; total: number }>;
  }

  async getStateInstance(resourceId: string): Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }> {
    return fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}`) as Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }>;
  }

  async createStateInstance(resourceId: string, resourceType: string): Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }> {
    return fetchJson('/__mockforge/api/state-machines/instances', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ resource_id: resourceId, resource_type: resourceType }),
    }) as Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }>;
  }

  async executeTransition(resourceId: string, toState: string, context?: Record<string, unknown>): Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }> {
    return fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}/transition`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ resource_id: resourceId, to_state: toState, context }),
    }) as Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }>;
  }

  async getNextStates(resourceId: string): Promise<{ next_states: string[] }> {
    return fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}/next-states`) as Promise<{ next_states: string[] }>;
  }

  async getCurrentState(resourceId: string): Promise<{ resource_id: string; current_state: string }> {
    return fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}/state`) as Promise<{ resource_id: string; current_state: string }>;
  }

  async exportStateMachines(): Promise<{ state_machines: unknown[]; visual_layouts: Record<string, unknown> }> {
    return fetchJson('/__mockforge/api/state-machines/export') as Promise<{ state_machines: unknown[]; visual_layouts: Record<string, unknown> }>;
  }

  async importStateMachines(data: { state_machines: unknown[]; visual_layouts: Record<string, unknown> }): Promise<void> {
    await fetchJson('/__mockforge/api/state-machines/import', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
  }
}

export { StateMachineApiMixin };
