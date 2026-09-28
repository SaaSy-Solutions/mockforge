/**
 * Pillar Analytics Page
 *
 * Displays pillar usage analytics dashboard for workspaces and organizations
 */

import React from 'react';
import { PillarAnalyticsDashboard } from '@/components/analytics/PillarAnalyticsDashboard';
import { useWorkspaceStore } from '@/stores/useWorkspaceStore';
import { useCloudOrgId } from '@/hooks/useCloudOrgId';
import { Card } from '@/components/ui/Card';

export const PillarAnalyticsPage: React.FC = () => {
  // Subscribe to the store rather than snapshotting it: workspaces load
  // asynchronously after sign-in, and a mount-time snapshot left the page
  // stuck with no scope (and the metrics query permanently disabled).
  const workspaceId = useWorkspaceStore((state) => state.activeWorkspace?.id);
  // Cloud mode: fall back to org-wide metrics when no workspace is selected.
  const orgId = useCloudOrgId() ?? undefined;

  if (!workspaceId && !orgId) {
    return (
      <div className="space-y-6 p-6">
        <Card className="p-6">
          <h2 className="text-lg font-semibold text-foreground mb-4">
            Select Workspace
          </h2>
          <p className="text-sm text-muted-foreground">
            Select a workspace to view pillar analytics.
          </p>
        </Card>
      </div>
    );
  }

  return <PillarAnalyticsDashboard workspaceId={workspaceId} orgId={orgId} />;
};

export default PillarAnalyticsPage;
