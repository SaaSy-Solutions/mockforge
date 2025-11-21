/**
 * ConsumerImpactPanel Component
 *
 * Displays consumer impact analysis for drift incidents, showing which
 * SDK methods and applications are affected by contract changes.
 */

import React, { useState, useEffect } from 'react';
import { Users, Smartphone, Globe, Code, Package, AlertCircle, ExternalLink, ChevronDown, ChevronRight } from 'lucide-react';
import { driftApi, type ConsumerImpact, type ConsumingApp, type SDKMethod, type AppType } from '../services/driftApi';
import { ModernCard, Alert } from './ui/DesignSystem';

interface ConsumerImpactPanelProps {
  incidentId: string;
  endpoint: string;
  method: string;
  affectedConsumers?: ConsumerImpact | null;
}

// App type icon mapping
const appTypeIcons: Record<AppType, React.ReactNode> = {
  web: <Globe className="w-4 h-4" />,
  mobile_ios: <Smartphone className="w-4 h-4" />,
  mobile_android: <Smartphone className="w-4 h-4" />,
  internal_tool: <Code className="w-4 h-4" />,
  cli: <Package className="w-4 h-4" />,
  other: <Users className="w-4 h-4" />,
};

// App type color mapping
const appTypeColors: Record<AppType, { bg: string; text: string; border: string }> = {
  web: {
    bg: 'bg-blue-50 dark:bg-blue-900/20',
    text: 'text-blue-700 dark:text-blue-300',
    border: 'border-blue-200 dark:border-blue-800',
  },
  mobile_ios: {
    bg: 'bg-gray-50 dark:bg-gray-800',
    text: 'text-gray-700 dark:text-gray-300',
    border: 'border-gray-200 dark:border-gray-700',
  },
  mobile_android: {
    bg: 'bg-green-50 dark:bg-green-900/20',
    text: 'text-green-700 dark:text-green-300',
    border: 'border-green-200 dark:border-green-800',
  },
  internal_tool: {
    bg: 'bg-purple-50 dark:bg-purple-900/20',
    text: 'text-purple-700 dark:text-purple-300',
    border: 'border-purple-200 dark:border-purple-800',
  },
  cli: {
    bg: 'bg-orange-50 dark:bg-orange-900/20',
    text: 'text-orange-700 dark:text-orange-300',
    border: 'border-orange-200 dark:border-orange-800',
  },
  other: {
    bg: 'bg-gray-50 dark:bg-gray-800',
    text: 'text-gray-700 dark:text-gray-300',
    border: 'border-gray-200 dark:border-gray-700',
  },
};

// Format app type for display
function formatAppType(appType: AppType): string {
  const labels: Record<AppType, string> = {
    web: 'Web App',
    mobile_ios: 'Mobile App (iOS)',
    mobile_android: 'Mobile App (Android)',
    internal_tool: 'Internal Tool',
    cli: 'CLI Tool',
    other: 'Other',
  };
  return labels[appType] || appType;
}

// App card component
function AppCard({ app }: { app: ConsumingApp }) {
  const colors = appTypeColors[app.app_type];
  const icon = appTypeIcons[app.app_type];

  return (
    <div className={`p-3 rounded-lg border ${colors.bg} ${colors.border} ${colors.text}`}>
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-2 flex-1">
          <div className="mt-0.5">{icon}</div>
          <div className="flex-1 min-w-0">
            <div className="font-medium text-sm truncate">{app.app_name}</div>
            <div className="text-xs mt-0.5 opacity-75">{formatAppType(app.app_type)}</div>
            {app.description && (
              <div className="text-xs mt-1 opacity-60 line-clamp-2">{app.description}</div>
            )}
          </div>
        </div>
        {app.repository_url && (
          <a
            href={app.repository_url}
            target="_blank"
            rel="noopener noreferrer"
            className="ml-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
            title="View repository"
          >
            <ExternalLink className="w-4 h-4" />
          </a>
        )}
      </div>
    </div>
  );
}

// SDK method card component
function SDKMethodCard({ sdkMethod, isExpanded, onToggle }: { sdkMethod: SDKMethod; isExpanded: boolean; onToggle: () => void }) {
  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
      <button
        onClick={onToggle}
        className="w-full px-4 py-3 bg-gray-50 dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors flex items-center justify-between text-left"
      >
        <div className="flex items-center gap-3">
          <div className="text-gray-500 dark:text-gray-400">
            {isExpanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          </div>
          <div>
            <div className="font-medium text-sm text-gray-900 dark:text-gray-100">
              {sdkMethod.sdk_name}
            </div>
            <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              {sdkMethod.method_name}
            </div>
          </div>
        </div>
        <div className="text-xs text-gray-500 dark:text-gray-400">
          {sdkMethod.consuming_apps.length} {sdkMethod.consuming_apps.length === 1 ? 'app' : 'apps'}
        </div>
      </button>
      {isExpanded && sdkMethod.consuming_apps.length > 0 && (
        <div className="p-4 bg-white dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700">
          <div className="space-y-2">
            {sdkMethod.consuming_apps.map((app) => (
              <AppCard key={app.app_id} app={app} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function ConsumerImpactPanel({ incidentId, endpoint, method, affectedConsumers }: ConsumerImpactPanelProps) {
  const [impact, setImpact] = useState<ConsumerImpact | null>(affectedConsumers || null);
  const [loading, setLoading] = useState(!affectedConsumers);
  const [error, setError] = useState<string | null>(null);
  const [expandedSDKMethods, setExpandedSDKMethods] = useState<Set<string>>(new Set());

  useEffect(() => {
    // If affected_consumers is provided directly, use it and skip API call
    if (affectedConsumers) {
      setImpact(affectedConsumers);
      setLoading(false);
      if (affectedConsumers.affected_sdk_methods.length > 0) {
        const firstKey = `${affectedConsumers.affected_sdk_methods[0].sdk_name}:${affectedConsumers.affected_sdk_methods[0].method_name}`;
        setExpandedSDKMethods(new Set([firstKey]));
      }
      return;
    }

    // Otherwise, fetch from API
    let cancelled = false;

    async function fetchImpact() {
      try {
        setLoading(true);
        setError(null);
        const response = await driftApi.getIncidentImpact(incidentId);
        if (!cancelled) {
          if (response.impact) {
            setImpact(response.impact);
            // Expand first SDK method by default
            if (response.impact.affected_sdk_methods.length > 0) {
              const firstKey = `${response.impact.affected_sdk_methods[0].sdk_name}:${response.impact.affected_sdk_methods[0].method_name}`;
              setExpandedSDKMethods(new Set([firstKey]));
            }
          } else {
            setImpact(null);
          }
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to load consumer impact');
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    fetchImpact();

    return () => {
      cancelled = true;
    };
  }, [incidentId, affectedConsumers]);

  const toggleSDKMethod = (sdkName: string, methodName: string) => {
    const key = `${sdkName}:${methodName}`;
    setExpandedSDKMethods((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };

  if (loading) {
    return (
      <ModernCard className="p-4">
        <div className="flex items-center gap-2 text-gray-600 dark:text-gray-400">
          <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-600 dark:border-gray-400"></div>
          <span className="text-sm">Loading consumer impact...</span>
        </div>
      </ModernCard>
    );
  }

  if (error) {
    return (
      <Alert variant="error" className="mt-4">
        <AlertCircle className="w-4 h-4" />
        <span>Failed to load consumer impact: {error}</span>
      </Alert>
    );
  }

  if (!impact) {
    return (
      <ModernCard className="p-4">
        <div className="flex items-center gap-2 text-gray-600 dark:text-gray-400">
          <Users className="w-4 h-4" />
          <span className="text-sm">No consumer mappings found for this endpoint</span>
        </div>
      </ModernCard>
    );
  }

  return (
    <ModernCard className="p-4 mt-4">
      <div className="flex items-start gap-2 mb-4">
        <Users className="w-5 h-5 text-blue-600 dark:text-blue-400 mt-0.5" />
        <div className="flex-1">
          <h3 className="font-semibold text-gray-900 dark:text-gray-100">Consumer Impact</h3>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">{impact.impact_summary}</p>
        </div>
      </div>

      {/* Affected Apps Summary */}
      {impact.affected_apps.length > 0 && (
        <div className="mb-4">
          <div className="text-xs font-medium text-gray-700 dark:text-gray-300 mb-2">
            Affected Applications ({impact.affected_apps.length})
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            {impact.affected_apps.map((app) => (
              <AppCard key={app.app_id} app={app} />
            ))}
          </div>
        </div>
      )}

      {/* SDK Methods */}
      {impact.affected_sdk_methods.length > 0 && (
        <div>
          <div className="text-xs font-medium text-gray-700 dark:text-gray-300 mb-2">
            Affected SDK Methods ({impact.affected_sdk_methods.length})
          </div>
          <div className="space-y-2">
            {impact.affected_sdk_methods.map((sdkMethod) => {
              const key = `${sdkMethod.sdk_name}:${sdkMethod.method_name}`;
              return (
                <SDKMethodCard
                  key={key}
                  sdkMethod={sdkMethod}
                  isExpanded={expandedSDKMethods.has(key)}
                  onToggle={() => toggleSDKMethod(sdkMethod.sdk_name, sdkMethod.method_name)}
                />
              );
            })}
          </div>
        </div>
      )}

      {/* Endpoint Info */}
      <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
        <div className="text-xs text-gray-500 dark:text-gray-400">
          <span className="font-medium">Endpoint:</span> {method} {endpoint}
        </div>
      </div>
    </ModernCard>
  );
}
