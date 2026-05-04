/**
 * Federation Detail Component
 *
 * Displays detailed information about a federation including services and routing
 */

import React, { useState } from 'react';
import { useFederation, useRouteRequest, type Federation } from '../../hooks/useFederation';
import { Card } from '../ui/Card';
import { ActiveScenarioPanel } from './ActiveScenarioPanel';
import { ArrowLeft, Edit, Network, Play, CheckCircle } from 'lucide-react';

export interface FederationDetailProps {
  federation: Federation;
  onEdit?: () => void;
  onBack?: () => void;
}

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'] as const;
type HttpMethod = (typeof HTTP_METHODS)[number];

export const FederationDetail: React.FC<FederationDetailProps> = ({
  federation: initialFederation,
  onEdit,
  onBack,
}) => {
  const { data: federation, isLoading } = useFederation(initialFederation.id);
  const routeRequest = useRouteRequest();
  const [testPath, setTestPath] = useState('');
  const [testMethod, setTestMethod] = useState<HttpMethod>('GET');
  const [testHeadersText, setTestHeadersText] = useState('');
  const [testBodyText, setTestBodyText] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [routingResult, setRoutingResult] = useState<any>(null);

  const currentFederation = federation || initialFederation;

  const parseHeaders = (text: string): Record<string, string> | null => {
    const trimmed = text.trim();
    if (!trimmed) return null;
    const headers: Record<string, string> = {};
    for (const line of trimmed.split('\n')) {
      const sep = line.indexOf(':');
      if (sep === -1) continue;
      const name = line.slice(0, sep).trim();
      const value = line.slice(sep + 1).trim();
      if (name) headers[name] = value;
    }
    return Object.keys(headers).length > 0 ? headers : null;
  };

  const parseBody = (text: string): unknown | undefined => {
    const trimmed = text.trim();
    if (!trimmed) return undefined;
    try {
      return JSON.parse(trimmed);
    } catch {
      return trimmed;
    }
  };

  const handleTestRoute = async () => {
    if (!testPath) return;

    const headers = parseHeaders(testHeadersText) ?? undefined;
    const body = parseBody(testBodyText);

    try {
      const result = await routeRequest.mutateAsync({
        federationId: currentFederation.id,
        request: {
          path: testPath,
          method: testMethod,
          headers,
          body,
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

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-info-600"></div>
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
              className="p-2 text-muted-foreground hover:text-foreground transition-colors"
            >
              <ArrowLeft className="h-5 w-5" />
            </button>
          )}
          <div>
            <h1 className="text-3xl font-bold text-foreground flex items-center gap-2">
              <Network className="h-8 w-8 text-info-600 dark:text-info-400" />
              {currentFederation.name}
            </h1>
            <p className="text-sm text-muted-foreground mt-1">
              {currentFederation.description || 'Federation Details'}
            </p>
          </div>
        </div>
        {onEdit && (
          <button
            onClick={onEdit}
            className="flex items-center gap-2 px-4 py-2 border border-border rounded-lg hover:bg-accent hover:text-accent-foreground transition-colors"
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
          <h3 className="text-lg font-semibold text-foreground mb-4">
            Basic Information
          </h3>
          <div className="space-y-3 text-sm">
            <div>
              <span className="text-muted-foreground">Federation ID:</span>
              <span className="ml-2 font-mono text-foreground">{currentFederation.id}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Organization ID:</span>
              <span className="ml-2 font-mono text-foreground">{currentFederation.org_id}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Created:</span>
              <span className="ml-2 text-foreground">
                {new Date(currentFederation.created_at).toLocaleString()}
              </span>
            </div>
            <div>
              <span className="text-muted-foreground">Updated:</span>
              <span className="ml-2 text-foreground">
                {new Date(currentFederation.updated_at).toLocaleString()}
              </span>
            </div>
          </div>
        </Card>

        {/* Services Summary */}
        <Card className="p-6">
          <h3 className="text-lg font-semibold text-foreground mb-4">
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
        <h3 className="text-lg font-semibold text-foreground mb-4">
          Services ({currentFederation.services.length})
        </h3>
        {currentFederation.services.length === 0 ? (
          <p className="text-sm text-muted-foreground">No services configured</p>
        ) : (
          <div className="space-y-4">
            {currentFederation.services.map((service, index) => (
              <div
                key={index}
                className="p-4 bg-muted rounded border-l-4 border-info-600"
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="font-medium text-foreground">
                    {service.name}
                  </div>
                  <span className={`px-2 py-1 rounded text-xs ${getRealityLevelColor(service.reality_level)}`}>
                    {service.reality_level}
                  </span>
                </div>
                <div className="space-y-1 text-sm text-muted-foreground">
                  <div>
                    <strong>Workspace ID:</strong>{' '}
                    <span className="font-mono">{service.workspace_id}</span>
                  </div>
                  <div>
                    <strong>Base Path:</strong> <code className="bg-muted dark:bg-gray-900 px-1 rounded">{service.base_path}</code>
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

      {/* Active Scenario */}
      <ActiveScenarioPanel federation={currentFederation} />

      {/* Test Routing */}
      <Card className="p-6">
        <h3 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
          <Play className="h-5 w-5" />
          Test Routing
        </h3>
        <div className="space-y-4">
          <div className="flex gap-2">
            <select
              value={testMethod}
              onChange={(e) => setTestMethod(e.target.value as HttpMethod)}
              className="px-3 py-2 border border-border rounded-lg bg-card text-foreground text-sm"
              aria-label="HTTP method"
            >
              {HTTP_METHODS.map((m) => (
                <option key={m} value={m}>{m}</option>
              ))}
            </select>
            <input
              type="text"
              value={testPath}
              onChange={(e) => setTestPath(e.target.value)}
              placeholder="/auth/login"
              className="flex-1 px-3 py-2 border border-border rounded-lg bg-card text-foreground"
            />
            <button
              onClick={handleTestRoute}
              disabled={!testPath || routeRequest.isPending}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Test Route
            </button>
          </div>
          <button
            type="button"
            onClick={() => setShowAdvanced((v) => !v)}
            className="text-xs text-info-600 dark:text-info-400 hover:underline"
          >
            {showAdvanced ? 'Hide' : 'Show'} headers & body (forward-compat with route handler)
          </button>
          {showAdvanced && (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div>
                <label className="block text-xs font-medium text-foreground mb-1">
                  Headers (one per line, <code>Name: value</code>)
                </label>
                <textarea
                  value={testHeadersText}
                  onChange={(e) => setTestHeadersText(e.target.value)}
                  rows={4}
                  placeholder={'Authorization: Bearer ...\nX-Trace-Id: abc'}
                  className="w-full px-3 py-2 font-mono text-xs border border-border rounded-lg bg-card text-foreground"
                />
              </div>
              <div>
                <label className="block text-xs font-medium text-foreground mb-1">
                  Body (JSON or raw text)
                </label>
                <textarea
                  value={testBodyText}
                  onChange={(e) => setTestBodyText(e.target.value)}
                  rows={4}
                  placeholder={'{"username": "alice"}'}
                  className="w-full px-3 py-2 font-mono text-xs border border-border rounded-lg bg-card text-foreground"
                />
              </div>
            </div>
          )}
          {routingResult && (
            <div className="p-4 bg-success-50 dark:bg-success-900 rounded">
              <div className="flex items-center gap-2 mb-2">
                <CheckCircle className="h-4 w-4 text-success-600 dark:text-success-400" />
                <span className="font-medium text-success-900 dark:text-success-200">Routing Successful</span>
              </div>
              <div className="text-sm text-success-700 dark:text-success-300 space-y-1">
                <div>
                  <strong>Service:</strong> {routingResult.service.name}
                </div>
                <div>
                  <strong>Workspace ID:</strong> <span className="font-mono">{routingResult.workspace_id}</span>
                </div>
                <div>
                  <strong>Service Path:</strong> <code className="bg-success-100 dark:bg-success-800 px-1 rounded">{routingResult.service_path}</code>
                </div>
              </div>
            </div>
          )}
        </div>
      </Card>
    </div>
  );
};
