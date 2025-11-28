/**
 * Pipeline Dashboard Component
 *
 * Main dashboard for managing MockOps pipelines
 */

import React, { useState } from 'react';
import { PipelineList } from './PipelineList';
import { PipelineDetail } from './PipelineDetail';
import { PipelineForm } from './PipelineForm';
import { PipelineExecutions } from './PipelineExecutions';
import { Card } from '../ui/Card';
import { Pipeline } from '../../hooks/usePipelines';

export interface PipelineDashboardProps {
  workspaceId?: string;
  orgId?: string;
}

type ViewMode = 'list' | 'create' | 'edit' | 'detail' | 'executions';

export const PipelineDashboard: React.FC<PipelineDashboardProps> = ({
  workspaceId,
  orgId,
}) => {
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [selectedPipeline, setSelectedPipeline] = useState<Pipeline | null>(null);

  const handleSelectPipeline = (pipeline: Pipeline) => {
    setSelectedPipeline(pipeline);
    setViewMode('detail');
  };

  const handleCreate = () => {
    setSelectedPipeline(null);
    setViewMode('create');
  };

  const handleEdit = (pipeline: Pipeline) => {
    setSelectedPipeline(pipeline);
    setViewMode('edit');
  };

  const handleBackToList = () => {
    setSelectedPipeline(null);
    setViewMode('list');
  };

  const handleViewExecutions = (pipeline: Pipeline) => {
    setSelectedPipeline(pipeline);
    setViewMode('executions');
  };

  return (
    <div className="space-y-6 p-6">
      {viewMode === 'list' && (
        <PipelineList
          workspaceId={workspaceId}
          orgId={orgId}
          onSelect={handleSelectPipeline}
          onCreate={handleCreate}
        />
      )}

      {viewMode === 'create' && (
        <PipelineForm
          workspaceId={workspaceId}
          orgId={orgId}
          onSave={() => setViewMode('list')}
          onCancel={handleBackToList}
        />
      )}

      {viewMode === 'edit' && selectedPipeline && (
        <PipelineForm
          workspaceId={workspaceId}
          orgId={orgId}
          pipeline={selectedPipeline}
          onSave={() => setViewMode('detail')}
          onCancel={() => setViewMode('detail')}
        />
      )}

      {viewMode === 'detail' && selectedPipeline && (
        <PipelineDetail
          pipeline={selectedPipeline}
          onEdit={() => handleEdit(selectedPipeline)}
          onViewExecutions={() => handleViewExecutions(selectedPipeline)}
          onBack={handleBackToList}
        />
      )}

      {viewMode === 'executions' && selectedPipeline && (
        <PipelineExecutions
          pipelineId={selectedPipeline.id}
          onBack={() => setViewMode('detail')}
        />
      )}
    </div>
  );
};
