/**
 * MockAI API service — OpenAPI generation, rule explanations, and learning from examples.
 */
import { fetchJson } from './client';

class MockAIApiMixin {
  async generateOpenApiFromTraffic(request: {
    database_path?: string;
    since?: string;
    until?: string;
    path_pattern?: string;
    min_confidence?: number;
  }): Promise<{
    spec: unknown;
    metadata: {
      requests_analyzed: number;
      paths_inferred: number;
      path_confidence: Record<string, { value: number; reason: string }>;
      generated_at: string;
      duration_ms: number;
    };
  }> {
    return fetchJson('/__mockforge/api/mockai/generate-openapi', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{
      spec: unknown;
      metadata: {
        requests_analyzed: number;
        paths_inferred: number;
        path_confidence: Record<string, { value: number; reason: string }>;
        generated_at: string;
        duration_ms: number;
      };
    }>;
  }

  async listRuleExplanations(filters?: {
    rule_type?: string;
    min_confidence?: number;
  }): Promise<{
    explanations: Array<{
      rule_id: string;
      rule_type: string;
      confidence: number;
      source_examples: string[];
      reasoning: string;
      pattern_matches: Array<{
        pattern: string;
        match_count: number;
        example_ids: string[];
      }>;
      generated_at: string;
    }>;
    total: number;
  }> {
    const params = new URLSearchParams();
    if (filters?.rule_type) {
      params.append('rule_type', filters.rule_type);
    }
    if (filters?.min_confidence !== undefined) {
      params.append('min_confidence', filters.min_confidence.toString());
    }
    const queryString = params.toString();
    const url = `/__mockforge/api/mockai/rules/explanations${queryString ? `?${queryString}` : ''}`;
    return fetchJson(url) as Promise<{
      explanations: Array<{
        rule_id: string;
        rule_type: string;
        confidence: number;
        source_examples: string[];
        reasoning: string;
        pattern_matches: Array<{
          pattern: string;
          match_count: number;
          example_ids: string[];
        }>;
        generated_at: string;
      }>;
      total: number;
    }>;
  }

  async getRuleExplanation(ruleId: string): Promise<{
    explanation: {
      rule_id: string;
      rule_type: string;
      confidence: number;
      source_examples: string[];
      reasoning: string;
      pattern_matches: Array<{
        pattern: string;
        match_count: number;
        example_ids: string[];
      }>;
      generated_at: string;
    };
  }> {
    return fetchJson(
      `/__mockforge/api/mockai/rules/${encodeURIComponent(ruleId)}/explanation`
    ) as Promise<{
      explanation: {
        rule_id: string;
        rule_type: string;
        confidence: number;
        source_examples: string[];
        reasoning: string;
        pattern_matches: Array<{
          pattern: string;
          match_count: number;
          example_ids: string[];
        }>;
        generated_at: string;
      };
    }>;
  }

  async learnFromExamples(request: {
    examples: Array<{
      request: unknown;
      response: unknown;
    }>;
    config?: unknown;
  }): Promise<{
    success: boolean;
    rules_generated: {
      consistency_rules: number;
      schemas: number;
      state_machines: number;
      system_prompt: boolean;
    };
    explanations: Array<{
      rule_id: string;
      rule_type: string;
      confidence: number;
      reasoning: string;
    }>;
    total_explanations: number;
  }> {
    return fetchJson('/__mockforge/api/mockai/learn', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{
      success: boolean;
      rules_generated: {
        consistency_rules: number;
        schemas: number;
        state_machines: number;
        system_prompt: boolean;
      };
      explanations: Array<{
        rule_id: string;
        rule_type: string;
        confidence: number;
        reasoning: string;
      }>;
      total_explanations: number;
    }>;
  }
}

export { MockAIApiMixin };
