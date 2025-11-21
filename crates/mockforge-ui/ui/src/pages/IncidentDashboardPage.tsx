import { logger } from '@/utils/logger';
import React, { useState, useMemo } from 'react';
import {
  AlertTriangle,
  CheckCircle2,
  Clock,
  RefreshCw,
  Search,
  XCircle,
  ExternalLink,
  Activity,
  TestTube,
  Network,
} from 'lucide-react';
import { ConsumerImpactPanel } from '../components/ConsumerImpactPanel';
import {
  useDriftIncidents,
  useDriftIncidentStatistics,
  useUpdateDriftIncident,
  useResolveDriftIncident,
} from '../hooks/useApi';
import type { DriftIncident, IncidentStatus, IncidentSeverity, IncidentType } from '../services/driftApi';
import {
  PageHeader,
  ModernCard,
  ModernBadge,
  Alert,
  EmptyState,
} from '../components/ui/DesignSystem';
import { Input } from '../components/ui/input';
import { Button } from '../components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../components/ui/select';

// Severity badge component
function SeverityBadge({ severity }: { severity: IncidentSeverity }) {
  const colors: Record<IncidentSeverity, { bg: string; text: string; icon: React.ReactNode }> = {
    critical: {
      bg: 'bg-red-100 dark:bg-red-900/20',
      text: 'text-red-800 dark:text-red-300',
      icon: <XCircle className="w-4 h-4" />,
    },
    high: {
      bg: 'bg-orange-100 dark:bg-orange-900/20',
      text: 'text-orange-800 dark:text-orange-300',
      icon: <AlertTriangle className="w-4 h-4" />,
    },
    medium: {
      bg: 'bg-yellow-100 dark:bg-yellow-900/20',
      text: 'text-yellow-800 dark:text-yellow-300',
      icon: <Clock className="w-4 h-4" />,
    },
    low: {
      bg: 'bg-blue-100 dark:bg-blue-900/20',
      text: 'text-blue-800 dark:text-blue-300',
      icon: <Activity className="w-4 h-4" />,
    },
  };

  const style = colors[severity] || colors.low;

  return (
    <span className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${style.bg} ${style.text}`}>
      {style.icon}
      {severity.toUpperCase()}
    </span>
  );
}

// Status badge component
function StatusBadge({ status }: { status: IncidentStatus }) {
  const colors: Record<IncidentStatus, { bg: string; text: string; icon: React.ReactNode }> = {
    open: {
      bg: 'bg-red-100 dark:bg-red-900/20',
      text: 'text-red-800 dark:text-red-300',
      icon: <AlertTriangle className="w-4 h-4" />,
    },
    acknowledged: {
      bg: 'bg-yellow-100 dark:bg-yellow-900/20',
      text: 'text-yellow-800 dark:text-yellow-300',
      icon: <Clock className="w-4 h-4" />,
    },
    resolved: {
      bg: 'bg-green-100 dark:bg-green-900/20',
      text: 'text-green-800 dark:text-green-300',
      icon: <CheckCircle2 className="w-4 h-4" />,
    },
    closed: {
      bg: 'bg-gray-100 dark:bg-gray-800',
      text: 'text-gray-800 dark:text-gray-300',
      icon: <CheckCircle2 className="w-4 h-4" />,
    },
  };

  const style = colors[status] || colors.open;

  return (
    <span className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${style.bg} ${style.text}`}>
      {style.icon}
      {status.charAt(0).toUpperCase() + status.slice(1)}
    </span>
  );
}

// Incident type badge
function IncidentTypeBadge({ type }: { type: IncidentType }) {
  const isBreaking = type === 'breaking_change';
  return (
    <span className={`px-2.5 py-1 rounded-full text-xs font-medium ${
      isBreaking
        ? 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-300'
        : 'bg-orange-100 text-orange-800 dark:bg-orange-900/20 dark:text-orange-300'
    }`}>
      {type === 'breaking_change' ? 'Breaking Change' : 'Threshold Exceeded'}
    </span>
  );
}

// Protocol-specific display component
function ProtocolDisplay({ 
  protocol, 
  endpoint, 
  method 
}: { 
  protocol: string; 
  endpoint: string; 
  method: string;
}) {
  // Parse protocol-specific information
  if (protocol === 'grpc') {
    // For gRPC, endpoint is typically "service.method" format
    const parts = endpoint.split('.');
    if (parts.length >= 2) {
      const service = parts.slice(0, -1).join('.');
      const methodName = parts[parts.length - 1];
      return (
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-500 dark:text-gray-400">gRPC</span>
          <span className="text-gray-400">•</span>
          <span className="font-mono text-sm font-semibold text-gray-900 dark:text-gray-100">
            {service}
          </span>
          <span className="text-gray-400">/</span>
          <span className="text-sm text-gray-700 dark:text-gray-300">
            {methodName}
          </span>
        </div>
      );
    }
  } else if (protocol === 'websocket') {
    // For WebSocket, endpoint might be the message type or channel
    return (
      <div className="flex items-center gap-2">
        <span className="text-xs text-gray-500 dark:text-gray-400">WebSocket</span>
        <span className="text-gray-400">•</span>
        <span className="text-sm text-gray-700 dark:text-gray-300 truncate">
          {endpoint || 'Message'}
        </span>
        {method && method !== 'websocket' && (
          <>
            <span className="text-gray-400">•</span>
            <span className="text-xs text-gray-500 dark:text-gray-400">
              {method}
            </span>
          </>
        )}
      </div>
    );
  } else if (protocol === 'mqtt' || protocol === 'kafka') {
    // For MQTT/Kafka, endpoint is the topic
    return (
      <div className="flex items-center gap-2">
        <span className="text-xs text-gray-500 dark:text-gray-400 uppercase">
          {protocol}
        </span>
        <span className="text-gray-400">•</span>
        <span className="font-mono text-sm font-semibold text-gray-900 dark:text-gray-100 truncate">
          Topic: {endpoint}
        </span>
      </div>
    );
  }
  
  // Fallback for other protocols
  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-gray-500 dark:text-gray-400 uppercase">
        {protocol}
      </span>
      <span className="text-gray-400">•</span>
      <span className="text-sm text-gray-700 dark:text-gray-300 truncate">
        {endpoint}
      </span>
    </div>
  );
}

// Incident row component
function IncidentRow({
  incident,
  onUpdate,
  onResolve,
}: {
  incident: DriftIncident;
  onUpdate: (id: string, status: IncidentStatus) => void;
  onResolve: (id: string) => void;
}) {
  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  return (
    <div className="border-b border-gray-200 dark:border-gray-700 last:border-b-0 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
      <div className="p-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-3 mb-2 flex-wrap">
              <div className="flex items-center gap-2">
                {/* Protocol-specific display */}
                {incident.protocol && incident.protocol !== 'http' ? (
                  <ProtocolDisplay 
                    protocol={incident.protocol} 
                    endpoint={incident.endpoint} 
                    method={incident.method} 
                  />
                ) : (
                  <>
                    <span className="font-mono text-sm font-semibold text-gray-900 dark:text-gray-100">
                      {incident.method}
                    </span>
                    <span className="text-gray-400">•</span>
                    <span className="text-sm text-gray-700 dark:text-gray-300 truncate">
                      {incident.endpoint}
                    </span>
                  </>
                )}
              </div>
              <SeverityBadge severity={incident.severity} />
              <StatusBadge status={incident.status} />
              <IncidentTypeBadge type={incident.incident_type} />
            </div>

            <div className="flex items-center gap-4 text-xs text-gray-500 dark:text-gray-400 mb-2">
              <span>Detected: {formatDate(incident.detected_at)}</span>
              {incident.resolved_at && (
                <span>Resolved: {formatDate(incident.resolved_at)}</span>
              )}
            </div>

            {incident.external_ticket_url && (
              <div className="mt-2">
                <a
                  href={incident.external_ticket_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1 text-xs text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300"
                >
                  <ExternalLink className="w-3 h-3" />
                  {incident.external_ticket_id || 'View Ticket'}
                </a>
              </div>
            )}

            {/* Protocol Badge - only show if not already displayed in header */}
            {incident.protocol && incident.protocol !== 'http' && (
              <div className="mt-2">
                <span className="inline-flex items-center px-2 py-1 rounded text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-300">
                  <Network className="w-3 h-3 mr-1" />
                  {incident.protocol.toUpperCase()}
                </span>
              </div>
            )}

            {/* Fitness Test Results */}
            {incident.fitness_test_results &&
             Array.isArray(incident.fitness_test_results) &&
             incident.fitness_test_results.length > 0 && (
              <details className="mt-2">
                <summary className="text-xs text-gray-600 dark:text-gray-400 cursor-pointer hover:text-gray-900 dark:hover:text-gray-200 flex items-center gap-1">
                  <TestTube className="w-3 h-3" />
                  Fitness Test Results ({incident.fitness_test_results.length})
                </summary>
                <div className="mt-2 space-y-2">
                  {incident.fitness_test_results.map((result, idx: number) => (
                    <div
                      key={idx}
                      className={`p-2 rounded text-xs ${
                        result.passed
                          ? 'bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800'
                          : 'bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800'
                      }`}
                    >
                      <div className="flex items-start justify-between mb-1">
                        <span className="font-semibold text-gray-900 dark:text-gray-100">
                          {result.function_name || `Test ${idx + 1}`}
                        </span>
                        {result.passed ? (
                          <CheckCircle2 className="w-4 h-4 text-green-600 dark:text-green-400 flex-shrink-0" />
                        ) : (
                          <XCircle className="w-4 h-4 text-red-600 dark:text-red-400 flex-shrink-0" />
                        )}
                      </div>
                      <p className="text-gray-700 dark:text-gray-300 mb-1">
                        {result.message}
                      </p>
                      {result.metrics && Object.keys(result.metrics).length > 0 && (
                        <div className="mt-1 pt-1 border-t border-gray-200 dark:border-gray-700">
                          <div className="grid grid-cols-2 gap-1">
                            {Object.entries(result.metrics).map(([key, value]) => (
                              <div key={key} className="text-xs">
                                <span className="text-gray-500 dark:text-gray-400">{key}:</span>{' '}
                                <span className="font-mono font-semibold">
                                  {typeof value === 'number' ? value.toFixed(2) : String(value)}
                                </span>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </details>
            )}

            {/* Protocol-specific drift details */}
            {incident.protocol && incident.protocol !== 'http' && incident.details && (
              <details className="mt-2">
                <summary className="text-xs text-gray-600 dark:text-gray-400 cursor-pointer hover:text-gray-900 dark:hover:text-gray-200 flex items-center gap-1">
                  <Network className="w-3 h-3" />
                  Protocol Drift Details
                </summary>
                <div className="mt-2 space-y-2">
                  {/* Show breaking vs additive changes if available */}
                  {typeof incident.details.breaking_changes === 'number' && (
                    <div className="p-2 rounded text-xs bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
                      <span className="font-semibold text-red-800 dark:text-red-300">
                        Breaking Changes: {incident.details.breaking_changes}
                      </span>
                    </div>
                  )}
                  {typeof incident.details.non_breaking_changes === 'number' && (
                    <div className="p-2 rounded text-xs bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800">
                      <span className="font-semibold text-green-800 dark:text-green-300">
                        Non-Breaking Changes: {incident.details.non_breaking_changes}
                      </span>
                    </div>
                  )}
                  {typeof incident.details.potentially_breaking_changes === 'number' && (
                    <div className="p-2 rounded text-xs bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800">
                      <span className="font-semibold text-yellow-800 dark:text-yellow-300">
                        Potentially Breaking: {incident.details.potentially_breaking_changes}
                      </span>
                    </div>
                  )}
                  {/* Show protocol-specific operation info */}
                  {incident.details.operation_id && (
                    <div className="text-xs text-gray-600 dark:text-gray-400">
                      <span className="font-semibold">Operation:</span> {String(incident.details.operation_id)}
                    </div>
                  )}
                  {incident.details.operation_type && (
                    <div className="text-xs text-gray-600 dark:text-gray-400">
                      <span className="font-semibold">Type:</span> {String(incident.details.operation_type)}
                    </div>
                  )}
                  {/* Show schema format if available */}
                  {incident.details.schema_format && (
                    <div className="p-2 rounded text-xs bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800">
                      <span className="font-semibold text-blue-800 dark:text-blue-300">
                        Schema Format: {String(incident.details.schema_format).replace(/_/g, ' ').toUpperCase()}
                      </span>
                    </div>
                  )}
                  {/* Show service/method info for gRPC */}
                  {incident.protocol === 'grpc' && incident.details.service && (
                    <div className="text-xs text-gray-600 dark:text-gray-400">
                      <span className="font-semibold">Service:</span> {String(incident.details.service)}
                      {incident.details.method && (
                        <> • <span className="font-semibold">Method:</span> {String(incident.details.method)}</>
                      )}
                    </div>
                  )}
                </div>
              </details>
            )}

            {Object.keys(incident.details).length > 0 && (
              <details className="mt-2">
                <summary className="text-xs text-gray-600 dark:text-gray-400 cursor-pointer hover:text-gray-900 dark:hover:text-gray-200">
                  View Full Details
                </summary>
                <pre className="mt-2 p-2 bg-gray-50 dark:bg-gray-900 rounded text-xs overflow-x-auto">
                  {JSON.stringify(incident.details, null, 2)}
                </pre>
              </details>
            )}

            {/* Consumer Impact Panel */}
            <div className="mt-4">
              <ConsumerImpactPanel
                incidentId={incident.id}
                endpoint={incident.endpoint}
                method={incident.method}
                affectedConsumers={incident.affected_consumers}
              />
            </div>
          </div>

          <div className="flex items-center gap-2">
            {incident.status === 'open' && (
              <>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => onUpdate(incident.id, 'acknowledged')}
                >
                  Acknowledge
                </Button>
                <Button
                  variant="default"
                  size="sm"
                  onClick={() => onResolve(incident.id)}
                  className="bg-green-600 hover:bg-green-700"
                >
                  Resolve
                </Button>
              </>
            )}
            {incident.status === 'acknowledged' && (
              <Button
                variant="default"
                size="sm"
                onClick={() => onResolve(incident.id)}
                className="bg-green-600 hover:bg-green-700"
              >
                Resolve
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

// Statistics cards component
function StatisticsCards({ statistics }: { statistics: any }) {
  if (!statistics) return null;

  const stats = statistics.statistics || statistics;

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <ModernCard className="p-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-gray-600 dark:text-gray-400">Total Incidents</p>
            <p className="text-2xl font-bold text-gray-900 dark:text-gray-100 mt-1">
              {stats.total || 0}
            </p>
          </div>
          <div className="p-3 bg-blue-100 dark:bg-blue-900/20 rounded-lg">
            <Activity className="w-6 h-6 text-blue-600 dark:text-blue-400" />
          </div>
        </div>
      </ModernCard>

      <ModernCard className="p-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-gray-600 dark:text-gray-400">Open</p>
            <p className="text-2xl font-bold text-red-600 dark:text-red-400 mt-1">
              {stats.by_status?.open || 0}
            </p>
          </div>
          <div className="p-3 bg-red-100 dark:bg-red-900/20 rounded-lg">
            <AlertTriangle className="w-6 h-6 text-red-600 dark:text-red-400" />
          </div>
        </div>
      </ModernCard>

      <ModernCard className="p-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-gray-600 dark:text-gray-400">Resolved</p>
            <p className="text-2xl font-bold text-green-600 dark:text-green-400 mt-1">
              {stats.by_status?.resolved || 0}
            </p>
          </div>
          <div className="p-3 bg-green-100 dark:bg-green-900/20 rounded-lg">
            <CheckCircle2 className="w-6 h-6 text-green-600 dark:text-green-400" />
          </div>
        </div>
      </ModernCard>

      <ModernCard className="p-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-gray-600 dark:text-gray-400">Critical</p>
            <p className="text-2xl font-bold text-red-600 dark:text-red-400 mt-1">
              {stats.by_severity?.critical || 0}
            </p>
          </div>
          <div className="p-3 bg-red-100 dark:bg-red-900/20 rounded-lg">
            <XCircle className="w-6 h-6 text-red-600 dark:text-red-400" />
          </div>
        </div>
      </ModernCard>
    </div>
  );
}

export function IncidentDashboardPage() {
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState<IncidentStatus | 'all'>('all');
  const [severityFilter, setSeverityFilter] = useState<IncidentSeverity | 'all'>('all');
  const [typeFilter, setTypeFilter] = useState<IncidentType | 'all'>('all');
  const [endpointFilter, setEndpointFilter] = useState('');
  const [protocolFilter, setProtocolFilter] = useState<string>('all');

  // Build filter params
  const filterParams = useMemo(() => {
    const params: any = {};
    if (statusFilter !== 'all') params.status = statusFilter;
    if (severityFilter !== 'all') params.severity = severityFilter;
    if (typeFilter !== 'all') params.incident_type = typeFilter;
    if (endpointFilter) params.endpoint = endpointFilter;
    // Note: Protocol filter is applied client-side since API may not support it yet
    return params;
  }, [statusFilter, severityFilter, typeFilter, endpointFilter]);

  // Fetch incidents with filters
  const {
    data: incidentsData,
    isLoading: incidentsLoading,
    error: incidentsError,
    refetch: refetchIncidents,
  } = useDriftIncidents(filterParams, { refetchInterval: 5000 });

  // Fetch statistics
  const { data: statsData, isLoading: statsLoading } = useDriftIncidentStatistics();

  // Mutations
  const updateMutation = useUpdateDriftIncident();
  const resolveMutation = useResolveDriftIncident();

  // Filter incidents by search term and protocol
  const filteredIncidents = useMemo(() => {
    if (!incidentsData?.incidents) return [];
    
    let filtered = incidentsData.incidents;
    
    // Apply protocol filter
    if (protocolFilter !== 'all') {
      filtered = filtered.filter((incident) => {
        if (protocolFilter === 'http') {
          return !incident.protocol || incident.protocol === 'http';
        }
        return incident.protocol === protocolFilter;
      });
    }
    
    // Apply search term filter
    if (searchTerm) {
      const search = searchTerm.toLowerCase();
      filtered = filtered.filter(
        (incident) =>
          incident.endpoint.toLowerCase().includes(search) ||
          incident.method.toLowerCase().includes(search) ||
          incident.id.toLowerCase().includes(search)
      );
    }
    
    return filtered;
  }, [incidentsData, searchTerm, protocolFilter]);

  const handleUpdateStatus = async (id: string, status: IncidentStatus) => {
    try {
      await updateMutation.mutateAsync({
        id,
        request: { status },
      });
    } catch (error) {
      logger.error('Failed to update incident', error);
      alert(`Failed to update incident: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const handleResolve = async (id: string) => {
    try {
      await resolveMutation.mutateAsync(id);
    } catch (error) {
      logger.error('Failed to resolve incident', error);
      alert(`Failed to resolve incident: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const incidents = filteredIncidents;
  const statistics = statsData?.statistics || statsData;

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title="Incident Dashboard"
        description="Monitor and manage contract drift incidents"
        icon={AlertTriangle}
      />

      {/* Statistics Cards */}
      {!statsLoading && statistics && <StatisticsCards statistics={statistics} />}

      {/* Filters */}
      <ModernCard className="p-4">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-6 gap-4">
          <div className="lg:col-span-2">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-4 h-4" />
              <Input
                placeholder="Search by endpoint, method, or ID..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="pl-10"
              />
            </div>
          </div>

          <Select value={statusFilter} onValueChange={(value) => setStatusFilter(value as IncidentStatus | 'all')}>
            <SelectTrigger>
              <SelectValue placeholder="Status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Statuses</SelectItem>
              <SelectItem value="open">Open</SelectItem>
              <SelectItem value="acknowledged">Acknowledged</SelectItem>
              <SelectItem value="resolved">Resolved</SelectItem>
              <SelectItem value="closed">Closed</SelectItem>
            </SelectContent>
          </Select>

          <Select value={severityFilter} onValueChange={(value) => setSeverityFilter(value as IncidentSeverity | 'all')}>
            <SelectTrigger>
              <SelectValue placeholder="Severity" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Severities</SelectItem>
              <SelectItem value="critical">Critical</SelectItem>
              <SelectItem value="high">High</SelectItem>
              <SelectItem value="medium">Medium</SelectItem>
              <SelectItem value="low">Low</SelectItem>
            </SelectContent>
          </Select>

          <Select value={typeFilter} onValueChange={(value) => setTypeFilter(value as IncidentType | 'all')}>
            <SelectTrigger>
              <SelectValue placeholder="Type" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Types</SelectItem>
              <SelectItem value="breaking_change">Breaking Change</SelectItem>
              <SelectItem value="threshold_exceeded">Threshold Exceeded</SelectItem>
            </SelectContent>
          </Select>

          <Select value={protocolFilter} onValueChange={setProtocolFilter}>
            <SelectTrigger>
              <SelectValue placeholder="Protocol" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Protocols</SelectItem>
              <SelectItem value="http">HTTP</SelectItem>
              <SelectItem value="grpc">gRPC</SelectItem>
              <SelectItem value="websocket">WebSocket</SelectItem>
              <SelectItem value="mqtt">MQTT</SelectItem>
              <SelectItem value="kafka">Kafka</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="mt-4 flex items-center gap-2">
          <Input
            placeholder="Filter by endpoint..."
            value={endpointFilter}
            onChange={(e) => setEndpointFilter(e.target.value)}
            className="max-w-xs"
          />
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              setSearchTerm('');
              setStatusFilter('all');
              setSeverityFilter('all');
              setTypeFilter('all');
              setEndpointFilter('');
              setProtocolFilter('all');
            }}
          >
            Clear Filters
          </Button>
          <Button variant="outline" size="sm" onClick={() => refetchIncidents()}>
            <RefreshCw className="w-4 h-4 mr-2" />
            Refresh
          </Button>
        </div>
      </ModernCard>

      {/* Incidents List */}
      <ModernCard>
        {incidentsLoading ? (
          <div className="flex items-center justify-center py-12">
            <div className="text-center">
              <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
              <p className="mt-4 text-gray-600 dark:text-gray-400">Loading incidents...</p>
            </div>
          </div>
        ) : incidentsError ? (
          <Alert variant="error" title="Error loading incidents">
            {incidentsError instanceof Error ? incidentsError.message : 'Unknown error occurred'}
          </Alert>
        ) : incidents.length === 0 ? (
          <EmptyState
            icon={CheckCircle2}
            title="No Incidents Found"
            description={
              searchTerm || statusFilter !== 'all' || severityFilter !== 'all' || typeFilter !== 'all' || endpointFilter || protocolFilter !== 'all'
                ? 'Try adjusting your filters to see more results'
                : 'All clear! No contract drift incidents detected.'
            }
          />
        ) : (
          <div className="divide-y divide-gray-200 dark:divide-gray-700">
            {incidents.map((incident) => (
              <IncidentRow
                key={incident.id}
                incident={incident}
                onUpdate={handleUpdateStatus}
                onResolve={handleResolve}
              />
            ))}
          </div>
        )}

        {/* Pagination info */}
        {incidentsData && incidentsData.total > 0 && (
          <div className="p-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50">
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Showing {incidents.length} of {incidentsData.total} incidents
            </p>
          </div>
        )}
      </ModernCard>
    </div>
  );
}
