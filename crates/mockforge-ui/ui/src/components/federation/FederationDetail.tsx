/**
 * Federation Detail Component
 *
 * Displays detailed information about a federation including services and routing
 */

import React, { useState } from 'react';
import { useFederation, useRouteRequest, Federation } from '../../hooks/useFederation';
import { Card } from '../ui/Card';
import { ArrowLeft, Edit, Network, Play, CheckCircle } from 'lucide-react';

export interface FederationDetailProps {
  federation: Federation;
  onEdit?: () => void;
  onBack?: () => void;
}

export const FederationDetail: React.FC<FederationDetailProps> = ({
  federation: initialFederation,
  onEdit,
  onBack,
}) => {
  const { data: federation, isLoading } = useFederation(initialFederation.id);
  const routeRequest = useRouteRequest();
  const [testPath, setTestPath] = useState('');
  const [routingResult, setRoutingResult] = useState<any>(null);

  const currentFederation = federation || initialFederation;

  const handleTestRoute = async () => {
    if (!testPath) return;

    try {
      const result = await routeRequest.mutateAsync({
        federationId: currentFederation.id,
        request: {
          path: testPath,
          method: 'GET',
        },
      });
      setRoutingResult(result);
    } catch (err) {
      alert(`Failed to route request: ${err instanceof Error ? err.message : 'Unknown error'}`);
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

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          {onBack && (
            <button
              onClick={onBack}
              className="p-2 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
            >
              <ArrowLeft className="h-5 w-5" />
            </button>
          )}
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
              <Network className="h-8 w-8 text-blue-600 dark:text-blue-400" />
              {currentFederation.name}
            </h1>
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
              {currentFederation.description || 'Federation Details'}
            </p>
          </div>
        </div>
        {onEdit && (
          <button
            onClick={onEdit}
            className="flex items-center gap-2 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
          >
            <Edit className="h-4 w-4" />
            Edit
          </button>
        )}
      </div>

      {/* Federation Information */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Basic Info */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Basic Information
          </h3>
          <div className="space-y-3 text-sm">
            <div>
              <span className="text-gray-600 dark:text-gray-400">Federation ID:</span>
              <span className="ml-2 font-mono text-gray-900 dark:text-white">{currentFederation.id}</span>
            </div>
            <div>
              <span className="text-gray-600 dark:text-gray-400">Organization ID:</span>
              <span className="ml-2 font-mono text-gray-900 dark:text-white">{currentFederation.org_id}</span>
            </div>
            <div>
              <span className="text-gray-600 dark:text-gray-400">Created:</span>
              <span className="ml-2 text-gray-900 dark:text-white">
                {new Date(currentFederation.created_at).toLocaleString()}
              </span>
            </div>
            <div>
              <span className="text-gray-600 dark:text-gray-400">Updated:</span>
              <span className="ml-2 text-gray-900 dark:text-white">
                {new Date(currentFederation.updated_at).toLocaleString()}
              </span>
            </div>
          </div>
        </Card>

        {/* Services Summary */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Services Summary
          </h3>
          <div className="space-y-2 text-sm">
            <div>
              <strong>Total Services:</strong> {currentFederation.services.length}
            </div>
            <div>
              <strong>Reality Levels:</strong>
              <div className="flex flex-wrap gap-2 mt-2">
                {['real', 'mock_v3', 'blended', 'chaos_driven'].map((level) => {
                  const count = currentFederation.services.filter(s => s.reality_level === level).length;
                  if (count === 0) return null;
                  return (
                    <span
                      key={level}
                      className={`px-2 py-1 rounded text-xs ${getRealityLevelColor(level)}`}
                    >
                      {level}: {count}
                    </span>
                  );
                })}
              </div>
            </div>
          </div>
        </Card>
      </div>

      {/* Services */}
      <Card className="p-6">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Services ({currentFederation.services.length})
        </h3>
        {currentFederation.services.length === 0 ? (
          <p className="text-sm text-gray-600 dark:text-gray-400">No services configured</p>
        ) : (
          <div className="space-y-4">
            {currentFederation.services.map((service, index) => (
              <div
                key={index}
                className="p-4 bg-gray-50 dark:bg-gray-800 rounded border-l-4 border-blue-600"
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="font-medium text-gray-900 dark:text-white">
                    {service.name}
                  </div>
                  <span className={`px-2 py-1 rounded text-xs ${getRealityLevelColor(service.reality_level)}`}>
                    {service.reality_level}
                  </span>
                </div>
                <div className="space-y-1 text-sm text-gray-600 dark:text-gray-400">
                  <div>
                    <strong>Workspace ID:</strong>{' '}
                    <span className="font-mono">{service.workspace_id}</span>
                  </div>
                  <div>
                    <strong>Base Path:</strong> <code className="bg-gray-100 dark:bg-gray-900 px-1 rounded">{service.base_path}</code>
                  </div>
                  {service.dependencies && service.dependencies.length > 0 && (
                    <div>
                      <strong>Dependencies:</strong>{' '}
                      {service.dependencies.join(', ')}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Test Routing */}
      <Card className="p-6">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          <Play className="h-5 w-5" />
          Test Routing
        </h3>
        <div className="space-y-4">
          <div className="flex gap-2">
            <input
              type="text"
              value={testPath}
              onChange={(e) => setTestPath(e.target.value)}
              placeholder="/auth/login"
              className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
            />
            <button
              onClick={handleTestRoute}
              disabled={!testPath || routeRequest.isPending}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Test Route
            </button>
          </div>
          {routingResult && (
            <div className="p-4 bg-green-50 dark:bg-green-900 rounded">
              <div className="flex items-center gap-2 mb-2">
                <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                <span className="font-medium text-green-900 dark:text-green-200">Routing Successful</span>
              </div>
              <div className="text-sm text-green-800 dark:text-green-300 space-y-1">
                <div>
                  <strong>Service:</strong> {routingResult.service.name}
                </div>
                <div>
                  <strong>Workspace ID:</strong> <span className="font-mono">{routingResult.workspace_id}</span>
                </div>
                <div>
                  <strong>Service Path:</strong> <code className="bg-green-100 dark:bg-green-800 px-1 rounded">{routingResult.service_path}</code>
                </div>
              </div>
            </div>
          )}
        </div>
      </Card>
    </div>
  );
};
