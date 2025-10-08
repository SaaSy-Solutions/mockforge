import React, { useState, useEffect } from 'react';
import { Search, Clock, AlertCircle, CheckCircle, XCircle } from 'lucide-react';
import {
  PageHeader,
  ModernCard,
  Alert,
  Section,
  ModernBadge
} from '../components/ui/DesignSystem';

interface Span {
  span_id: string;
  trace_id: string;
  parent_span_id?: string;
  name: string;
  kind: string;
  start_time: string;
  end_time: string;
  duration_ms: number;
  status: 'ok' | 'error';
  attributes: Record<string, any>;
  events: Array<{
    name: string;
    timestamp: string;
    attributes: Record<string, any>;
  }>;
}

interface Trace {
  trace_id: string;
  spans: Span[];
  start_time: string;
  end_time: string;
  duration_ms: number;
  service_name: string;
  status: 'ok' | 'error';
}

export function TracesPage() {
  const [traces, setTraces] = useState<Trace[]>([]);
  const [selectedTrace, setSelectedTrace] = useState<Trace | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    fetchTraces();
  }, []);

  const fetchTraces = async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await fetch('/api/observability/traces');
      if (!response.ok) throw new Error('Failed to fetch traces');
      const data = await response.json();
      setTraces(data.traces || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  const filteredTraces = traces.filter(trace =>
    trace.trace_id.includes(searchQuery) ||
    trace.service_name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const renderSpanTree = (span: Span, level: number = 0) => {
    const childSpans = selectedTrace?.spans.filter(s => s.parent_span_id === span.span_id) || [];

    return (
      <div key={span.span_id}>
        <div
          className="flex items-center py-2 px-3 hover:bg-gray-50 dark:hover:bg-gray-800 rounded-lg cursor-pointer"
          style={{ paddingLeft: `${level * 1.5 + 0.75}rem` }}
        >
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              {span.status === 'ok' ? (
                <CheckCircle className="h-4 w-4 text-green-500 flex-shrink-0" />
              ) : (
                <XCircle className="h-4 w-4 text-red-500 flex-shrink-0" />
              )}
              <span className="font-medium text-gray-900 dark:text-gray-100 truncate">
                {span.name}
              </span>
              <ModernBadge size="sm" variant="default">{span.kind}</ModernBadge>
            </div>
            <div className="flex items-center gap-4 mt-1 text-xs text-gray-500 dark:text-gray-400">
              <span className="font-mono">{span.span_id.substring(0, 8)}</span>
              <span>{span.duration_ms.toFixed(2)}ms</span>
            </div>
          </div>
        </div>
        {childSpans.map(child => renderSpanTree(child, level + 1))}
      </div>
    );
  };

  if (loading) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Distributed Traces"
          subtitle="OpenTelemetry trace viewer"
        />
        <Alert type="info" title="Loading traces" message="Fetching trace data..." />
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Distributed Traces"
          subtitle="OpenTelemetry trace viewer"
        />
        <Alert
          type="error"
          title="Failed to load traces"
          message={error}
          actions={
            <button
              onClick={fetchTraces}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
            >
              Retry
            </button>
          }
        />
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <PageHeader
        title="Distributed Traces"
        subtitle="View and analyze OpenTelemetry traces"
        actions={
          <button
            onClick={fetchTraces}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            Refresh
          </button>
        }
      />

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-5 w-5 text-gray-400" />
        <input
          type="text"
          placeholder="Search traces by ID or service name..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100"
        />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {/* Traces List */}
        <Section title="Traces" subtitle={`${filteredTraces.length} traces found`}>
          <ModernCard>
            {filteredTraces.length === 0 ? (
              <div className="text-center py-8">
                <AlertCircle className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                <p className="text-gray-500 dark:text-gray-400">
                  No traces found. Ensure OpenTelemetry tracing is enabled.
                </p>
              </div>
            ) : (
              <div className="space-y-2 max-h-[600px] overflow-y-auto">
                {filteredTraces.map(trace => (
                  <div
                    key={trace.trace_id}
                    onClick={() => setSelectedTrace(trace)}
                    className={`p-4 rounded-lg cursor-pointer border ${
                      selectedTrace?.trace_id === trace.trace_id
                        ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                        : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <span className="font-mono text-sm text-gray-900 dark:text-gray-100">
                        {trace.trace_id.substring(0, 16)}...
                      </span>
                      <ModernBadge variant={trace.status === 'ok' ? 'success' : 'error'} size="sm">
                        {trace.status}
                      </ModernBadge>
                    </div>
                    <div className="flex items-center gap-4 text-xs text-gray-500 dark:text-gray-400">
                      <span>{trace.service_name}</span>
                      <span className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        {trace.duration_ms.toFixed(2)}ms
                      </span>
                      <span>{trace.spans.length} spans</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </ModernCard>
        </Section>

        {/* Trace Details */}
        <Section title="Trace Details" subtitle={selectedTrace ? `${selectedTrace.spans.length} spans` : 'Select a trace'}>
          <ModernCard>
            {!selectedTrace ? (
              <div className="text-center py-8">
                <p className="text-gray-500 dark:text-gray-400">
                  Select a trace to view details
                </p>
              </div>
            ) : (
              <div className="space-y-6">
                {/* Trace Info */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-500 dark:text-gray-400">Trace ID</span>
                    <span className="font-mono text-sm">{selectedTrace.trace_id}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-500 dark:text-gray-400">Duration</span>
                    <span className="font-mono text-sm">{selectedTrace.duration_ms.toFixed(2)}ms</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-500 dark:text-gray-400">Service</span>
                    <span className="font-mono text-sm">{selectedTrace.service_name}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-gray-500 dark:text-gray-400">Status</span>
                    <ModernBadge variant={selectedTrace.status === 'ok' ? 'success' : 'error'}>
                      {selectedTrace.status}
                    </ModernBadge>
                  </div>
                </div>

                <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
                  <h4 className="font-semibold text-gray-900 dark:text-gray-100 mb-4">Span Tree</h4>
                  <div className="space-y-1 max-h-[400px] overflow-y-auto">
                    {selectedTrace.spans
                      .filter(span => !span.parent_span_id)
                      .map(span => renderSpanTree(span))}
                  </div>
                </div>
              </div>
            )}
          </ModernCard>
        </Section>
      </div>
    </div>
  );
}
