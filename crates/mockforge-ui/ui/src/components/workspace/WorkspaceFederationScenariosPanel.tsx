/**
 * Workspace Federation Scenarios Panel
 *
 * Read-only diagnostic that surfaces every federation scenario currently
 * applying to a workspace. Backed by `GET /api/v1/workspaces/{id}/active-scenarios`,
 * which the runtime poller also consumes — the admin UI uses it to answer
 * "which active federation overrides are landing on this workspace right now?"
 *
 * Renders nothing when the workspace is not participating in any active
 * federation scenario, so workspaces unaware of federation stay uncluttered.
 */

import React from 'react';
import { Zap } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Badge } from '../ui/Badge';
import {
  useWorkspaceActiveFederationScenarios,
  type WorkspaceActiveScenarioEntry,
} from '../../hooks/useFederation';

export interface WorkspaceFederationScenariosPanelProps {
  workspaceId: string;
}

const WorkspaceFederationScenariosPanel: React.FC<WorkspaceFederationScenariosPanelProps> = ({
  workspaceId,
}) => {
  const { data, isLoading, isError } = useWorkspaceActiveFederationScenarios(workspaceId);

  if (isLoading || isError) return null;
  const entries = data?.entries ?? [];
  if (entries.length === 0) return null;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Zap className="h-4 w-4 text-amber-500" />
          Active Federation Scenarios
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {entries.map((entry) => (
            <ScenarioEntryRow key={`${entry.activation_id}-${entry.service_name}`} entry={entry} />
          ))}
        </div>
      </CardContent>
    </Card>
  );
};

const ScenarioEntryRow: React.FC<{ entry: WorkspaceActiveScenarioEntry }> = ({ entry }) => {
  const override = entry.override_config ?? null;
  return (
    <div className="p-3 bg-muted/40 rounded border border-border">
      <div className="flex items-center justify-between mb-1 gap-2">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="font-medium text-sm">{entry.scenario_name}</span>
          <span className="text-xs text-muted-foreground">via</span>
          <span className="text-sm font-medium">{entry.federation_name || entry.federation_id}</span>
        </div>
        <Badge variant="outline" className="text-xs">
          service: {entry.service_name}
        </Badge>
      </div>
      {override && (
        <div className="flex flex-wrap gap-2 mt-1 text-xs text-muted-foreground">
          {override.reality_level && (
            <span className="px-2 py-0.5 bg-background rounded border border-border">
              reality: {override.reality_level}
            </span>
          )}
          {override.chaos_level !== undefined && (
            <span className="px-2 py-0.5 bg-background rounded border border-border">
              chaos: {override.chaos_level}
            </span>
          )}
          {override.failure_rate !== undefined && (
            <span className="px-2 py-0.5 bg-background rounded border border-border">
              failure: {override.failure_rate}
            </span>
          )}
          {override.latency_ms !== undefined && (
            <span className="px-2 py-0.5 bg-background rounded border border-border">
              latency: {override.latency_ms}ms
            </span>
          )}
          {override.notes && (
            <span className="px-2 py-0.5 bg-background rounded border border-border italic">
              {override.notes}
            </span>
          )}
        </div>
      )}
    </div>
  );
};

export default WorkspaceFederationScenariosPanel;
