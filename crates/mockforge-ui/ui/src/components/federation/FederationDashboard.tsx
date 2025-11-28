/**
 * Federation Dashboard Component
 *
 * Main dashboard for managing MockOps federations
 */

import React, { useState } from 'react';
import { FederationList } from './FederationList';
import { FederationDetail } from './FederationDetail';
import { FederationForm } from './FederationForm';
import { Federation } from '../../hooks/useFederation';

export interface FederationDashboardProps {
  orgId: string;
}

type ViewMode = 'list' | 'create' | 'edit' | 'detail';

export const FederationDashboard: React.FC<FederationDashboardProps> = ({
  orgId,
}) => {
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [selectedFederation, setSelectedFederation] = useState<Federation | null>(null);

  const handleSelectFederation = (federation: Federation) => {
    setSelectedFederation(federation);
    setViewMode('detail');
  };

  const handleCreate = () => {
    setSelectedFederation(null);
    setViewMode('create');
  };

  const handleEdit = (federation: Federation) => {
    setSelectedFederation(federation);
    setViewMode('edit');
  };

  const handleBackToList = () => {
    setSelectedFederation(null);
    setViewMode('list');
  };

  return (
    <div className="space-y-6 p-6">
      {viewMode === 'list' && (
        <FederationList
          orgId={orgId}
          onSelect={handleSelectFederation}
          onCreate={handleCreate}
        />
      )}

      {viewMode === 'create' && (
        <FederationForm
          orgId={orgId}
          onSave={() => setViewMode('list')}
          onCancel={handleBackToList}
        />
      )}

      {viewMode === 'edit' && selectedFederation && (
        <FederationForm
          orgId={orgId}
          federation={selectedFederation}
          onSave={() => setViewMode('detail')}
          onCancel={() => setViewMode('detail')}
        />
      )}

      {viewMode === 'detail' && selectedFederation && (
        <FederationDetail
          federation={selectedFederation}
          onEdit={() => handleEdit(selectedFederation)}
          onBack={handleBackToList}
        />
      )}
    </div>
  );
};
