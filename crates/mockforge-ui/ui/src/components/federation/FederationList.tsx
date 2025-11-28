/**
 * Federation List Component
 *
 * Displays a list of all federations with their services and actions
 */

import React from 'react';
import { useFederations, useDeleteFederation, Federation } from '../../hooks/useFederation';
import { Card } from '../ui/Card';
import { Edit, Trash2, Plus, Network, ArrowRight } from 'lucide-react';

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

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card className="p-6">
        <div className="text-red-600 dark:text-red-400">
          Error loading federations: {error.message}
        </div>
      </Card>
    );
  }

  const handleDelete = async (id: string, name: string) => {
    if (window.confirm(`Are you sure you want to delete federation "${name}"?`)) {
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
        return 'bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200';
      case 'mock_v3':
        return 'bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200';
      case 'blended':
        return 'bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-200';
      case 'chaos_driven':
        return 'bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-200';
      default:
        return 'bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200';
    }
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">Federations</h2>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            Compose multiple workspaces into federated virtual systems
          </p>
        </div>
        {onCreate && (
          <button
            onClick={onCreate}
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          >
            <Plus className="h-4 w-4" />
            Create Federation
          </button>
        )}
      </div>

      {/* Federation List */}
      {!federations || federations.length === 0 ? (
        <Card className="p-8 text-center">
          <p className="text-gray-600 dark:text-gray-400 mb-4">No federations found</p>
          {onCreate && (
            <button
              onClick={onCreate}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
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
                    <Network className="h-5 w-5 text-blue-600 dark:text-blue-400" />
                    <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                      {federation.name}
                    </h3>
                  </div>

                  {federation.description && (
                    <p className="text-sm text-gray-600 dark:text-gray-400 mb-3">
                      {federation.description}
                    </p>
                  )}

                  <div className="space-y-2">
                    <div className="text-sm text-gray-600 dark:text-gray-400">
                      <strong>Services:</strong> {federation.services.length}
                    </div>

                    <div className="flex flex-wrap gap-2">
                      {federation.services.map((service) => (
                        <div
                          key={service.name}
                          className="flex items-center gap-2 px-3 py-1 bg-gray-50 dark:bg-gray-800 rounded text-sm"
                        >
                          <span className="font-medium text-gray-900 dark:text-white">
                            {service.name}
                          </span>
                          <ArrowRight className="h-3 w-3 text-gray-400" />
                          <span className="text-gray-600 dark:text-gray-400">
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

                    <div className="text-xs text-gray-500 dark:text-gray-500 mt-2">
                      Updated {new Date(federation.updated_at).toLocaleDateString()}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2 ml-4">
                  {onSelect && (
                    <button
                      onClick={() => onSelect(federation)}
                      className="p-2 text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
                      title="View Details"
                    >
                      <Edit className="h-4 w-4" />
                    </button>
                  )}
                  <button
                    onClick={() => handleDelete(federation.id, federation.name)}
                    className="p-2 text-gray-600 dark:text-gray-400 hover:text-red-600 dark:hover:text-red-400 transition-colors"
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
