/**
 * Federation List Component
 *
 * Displays a list of all federations with their services and actions
 */

import React from 'react';
import {
  useActiveFederationScenario,
  useDeleteFederation,
  useFederations,
  type Federation,
} from '../../hooks/useFederation';
import { useConfirmDelete } from '../../hooks/useConfirmDelete';
import { Card } from '../ui/Card';
import { Edit, Trash2, Plus, Network, ArrowRight, Zap } from 'lucide-react';

export interface FederationListProps {
  orgId: string;
  onSelect?: (federation: Federation) => void;
  onCreate?: () => void;
}

export const FederationList: React.FC<FederationListProps> = ({
  orgId,
  onSelect,
  onCreate,
}) => {
  const { data: federations, isLoading, error } = useFederations(orgId);
  const deleteFederation = useDeleteFederation();
  const confirmDelete = useConfirmDelete();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-info-600"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card className="p-6">
        <div className="text-danger-600 dark:text-danger-400">
          Error loading federations: {error.message}
        </div>
      </Card>
    );
  }

  const handleDelete = async (id: string, name: string) => {
    if (confirmDelete(`Are you sure you want to delete federation "${name}"?`)) {
      try {
        await deleteFederation.mutateAsync(id);
      } catch (err) {
        alert(`Failed to delete federation: ${err instanceof Error ? err.message : 'Unknown error'}`);
      }
    }
  };

  const getRealityLevelColor = (level: string) => {
    switch (level) {
      case 'real':
        return 'bg-info-100 text-info-700 dark:bg-info-900/30 dark:text-info-300';
      case 'mock_v3':
        return 'bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-300';
      case 'blended':
        return 'bg-warning-100 text-warning-700 dark:bg-warning-900/30 dark:text-warning-300';
      case 'chaos_driven':
        return 'bg-danger-100 text-danger-700 dark:bg-danger-900/30 dark:text-danger-300';
      default:
        return 'bg-muted text-foreground';
    }
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-foreground">Federations</h2>
          <p className="text-sm text-muted-foreground mt-1">
            Compose multiple workspaces into federated virtual systems
          </p>
        </div>
        {onCreate && (
          <button
            onClick={onCreate}
            className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
          >
            <Plus className="h-4 w-4" />
            Create Federation
          </button>
        )}
      </div>

      {/* Federation List */}
      {!federations || federations.length === 0 ? (
        <Card className="p-8 text-center">
          <p className="text-muted-foreground mb-4">No federations found</p>
          {onCreate && (
            <button
              onClick={onCreate}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
            >
              Create Your First Federation
            </button>
          )}
        </Card>
      ) : (
        <div className="grid gap-4">
          {federations.map((federation) => (
            <Card key={federation.id} className="p-6 hover:shadow-lg transition-shadow">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-3 mb-2">
                    <Network className="h-5 w-5 text-info-600 dark:text-info-400" />
                    <h3 className="text-lg font-semibold text-foreground">
                      {federation.name}
                    </h3>
                  </div>

                  {federation.description && (
                    <p className="text-sm text-muted-foreground mb-3">
                      {federation.description}
                    </p>
                  )}

                  <ActiveScenarioBadge federationId={federation.id} />

                  <div className="space-y-2">
                    <div className="text-sm text-muted-foreground">
                      <strong>Services:</strong> {federation.services.length}
                    </div>

                    <div className="flex flex-wrap gap-2">
                      {federation.services.map((service) => (
                        <div
                          key={service.name}
                          className="flex items-center gap-2 px-3 py-1 bg-muted rounded text-sm"
                        >
                          <span className="font-medium text-foreground">
                            {service.name}
                          </span>
                          <ArrowRight className="h-3 w-3 text-muted-foreground" />
                          <span className="text-muted-foreground">
                            {service.base_path}
                          </span>
                          <span
                            className={`px-2 py-0.5 rounded text-xs ${getRealityLevelColor(service.reality_level)}`}
                          >
                            {service.reality_level}
                          </span>
                        </div>
                      ))}
                    </div>

                    <div className="text-xs text-muted-foreground mt-2">
                      Updated {new Date(federation.updated_at).toLocaleDateString()}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2 ml-4">
                  {onSelect && (
                    <button
                      onClick={() => onSelect(federation)}
                      className="p-2 text-muted-foreground hover:text-info-600 dark:hover:text-info-400 transition-colors"
                      title="View Details"
                    >
                      <Edit className="h-4 w-4" />
                    </button>
                  )}
                  <button
                    onClick={() => handleDelete(federation.id, federation.name)}
                    className="p-2 text-muted-foreground hover:text-danger-600 dark:hover:text-danger-400 transition-colors"
                    title="Delete Federation"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
};

interface ActiveScenarioBadgeProps {
  federationId: string;
}

/**
 * Inline badge that surfaces when a federation has an active scenario, so the
 * user doesn't have to click into each federation to know whether one is
 * running. Polling is disabled here — list-view callers shouldn't open a
 * poller per card. The cache is shared with the detail view's hook (same
 * query key), so opening a federation hydrates this badge instantly.
 */
const ActiveScenarioBadge: React.FC<ActiveScenarioBadgeProps> = ({ federationId }) => {
  const { data } = useActiveFederationScenario(federationId, { refetchInterval: false });
  if (!data) return null;
  return (
    <div className="mb-3 inline-flex items-center gap-2 px-3 py-1 bg-amber-50 dark:bg-amber-900/30 text-amber-800 dark:text-amber-200 rounded-full text-xs">
      <Zap className="h-3 w-3" />
      <span className="font-medium">Active scenario:</span>
      <span>{data.scenario_name}</span>
    </div>
  );
};
