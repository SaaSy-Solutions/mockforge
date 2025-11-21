/**
 * Pillar Analytics Page
 * 
 * Displays pillar usage analytics dashboard for workspaces and organizations
 */

import React, { useState } from 'react';
import { PillarAnalyticsDashboard } from '@/components/analytics/PillarAnalyticsDashboard';
import { useWorkspaceStore } from '@/stores/useWorkspaceStore';
import { Card } from '@/components/ui/Card';

export const PillarAnalyticsPage: React.FC = () => {
  const { currentWorkspace } = useWorkspaceStore();
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | undefined>(
    currentWorkspace?.id
  );

  return (
    <div className="space-y-6 p-6">
      {/* Workspace selector if needed */}
      {!selectedWorkspaceId && (
        <Card className="p-6">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Select Workspace
          </h2>
          <p className="text-sm text-gray-600 dark:text-gray-400">
            Please select a workspace to view pillar analytics, or view organization-wide metrics.
          </p>
        </Card>
      )}

      {/* Pillar Analytics Dashboard */}
      <PillarAnalyticsDashboard workspaceId={selectedWorkspaceId} />
    </div>
  );
};

export default PillarAnalyticsPage;

