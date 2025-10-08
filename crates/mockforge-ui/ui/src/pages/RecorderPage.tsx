import React, { useState, useEffect } from 'react';
import { Play, Download, Search, Filter, Clock } from 'lucide-react';
import {
  PageHeader,
  ModernCard,
  Alert,
  Section,
  ModernBadge
} from '../components/ui/DesignSystem';

interface RecordedRequest {
  id: number;
  timestamp: string;
  protocol: string;
  method: string;
  path: string;
  status_code: number;
  request_headers: Record<string, string>;
  request_body?: string;
  response_headers: Record<string, string>;
  response_body?: string;
  duration_ms: number;
  client_ip?: string;
}

interface RecordedScenario {
  name: string;
  started_at: string;
  ended_at?: string;
  total_events: number;
  duration_ms: number;
}

export function RecorderPage() {
  const [requests, setRequests] = useState<RecordedRequest[]>([]);
  const [scenarios, setScenarios] = useState<RecordedScenario[]>([]);
  const [selectedRequest, setSelectedRequest] = useState<RecordedRequest | null>(null);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [protocolFilter, setProtocolFilter] = useState<string>('all');
  const [isRecording, setIsRecording] = useState(false);

  useEffect(() => {
    fetchRequests();
    fetchScenarios();
    checkRecordingStatus();
  }, []);

  const fetchRequests = async () => {
    try {
      setLoading(true);
      const response = await fetch('/api/recorder/search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ limit: 100 })
      });
      if (!response.ok) throw new Error('Failed to fetch requests');
      const data = await response.json();
      setRequests(data.requests || []);
    } catch (err) {
      console.error('Failed to fetch requests:', err);
    } finally {
      setLoading(false);
    }
  };

  const fetchScenarios = async () => {
    try {
      const response = await fetch('/api/chaos/recording/list');
      if (!response.ok) throw new Error('Failed to fetch scenarios');
      const data = await response.json();
      setScenarios(data.scenarios || []);
    } catch (err) {
      console.error('Failed to fetch scenarios:', err);
    }
  };

  const checkRecordingStatus = async () => {
    try {
      const response = await fetch('/api/chaos/recording/status');
      if (!response.ok) return;
      const data = await response.json();
      setIsRecording(data.is_recording || false);
    } catch (err) {
      console.error('Failed to check recording status:', err);
    }
  };

  const startRecording = async () => {
    try {
      const response = await fetch('/api/chaos/recording/start', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ scenario_name: 'manual_recording' })
      });
      if (!response.ok) throw new Error('Failed to start recording');
      setIsRecording(true);
    } catch (err) {
      alert(`Failed to start recording: ${err}`);
    }
  };

  const stopRecording = async () => {
    try {
      const response = await fetch('/api/chaos/recording/stop', {
        method: 'POST',
      });
      if (!response.ok) throw new Error('Failed to stop recording');
      setIsRecording(false);
      fetchScenarios();
    } catch (err) {
      alert(`Failed to stop recording: ${err}`);
    }
  };

  const replayScenario = async (scenarioName: string) => {
    try {
      const response = await fetch('/api/chaos/replay/start', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          scenario_name: scenarioName,
          speed: 1.0,
          loop_replay: false
        })
      });
      if (!response.ok) throw new Error('Failed to start replay');
      alert('Replay started successfully');
    } catch (err) {
      alert(`Failed to start replay: ${err}`);
    }
  };

  const exportScenario = async (scenarioName: string) => {
    try {
      const response = await fetch('/api/chaos/recording/export', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          scenario_name: scenarioName,
          format: 'json'
        })
      });
      if (!response.ok) throw new Error('Failed to export scenario');
      const blob = await response.blob();
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${scenarioName}.json`;
      a.click();
    } catch (err) {
      alert(`Failed to export scenario: ${err}`);
    }
  };

  const filteredRequests = requests.filter(req => {
    const matchesSearch = req.path.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         req.method.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesProtocol = protocolFilter === 'all' || req.protocol === protocolFilter;
    return matchesSearch && matchesProtocol;
  });

  return (
    <div className="space-y-8">
      <PageHeader
        title="API Flight Recorder"
        subtitle="Record, replay, and analyze API interactions"
        actions={
          <div className="flex gap-2">
            {isRecording ? (
              <button
                onClick={stopRecording}
                className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 flex items-center gap-2"
              >
                <div className="h-2 w-2 bg-white rounded-full animate-pulse" />
                Stop Recording
              </button>
            ) : (
              <button
                onClick={startRecording}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 flex items-center gap-2"
              >
                <Play className="h-4 w-4" />
                Start Recording
              </button>
            )}
          </div>
        }
      />

      {isRecording && (
        <Alert
          type="warning"
          title="Recording in Progress"
          message="All API requests are being recorded. Stop recording to save the scenario."
        />
      )}

      {/* Recorded Scenarios */}
      {scenarios.length > 0 && (
        <Section
          title="Recorded Scenarios"
          subtitle={`${scenarios.length} scenarios available`}
        >
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {scenarios.map(scenario => (
              <ModernCard key={scenario.name}>
                <div className="mb-4">
                  <h3 className="font-semibold text-gray-900 dark:text-gray-100 mb-2">
                    {scenario.name}
                  </h3>
                  <div className="space-y-1 text-sm text-gray-500 dark:text-gray-400">
                    <div className="flex items-center gap-2">
                      <Clock className="h-3 w-3" />
                      {new Date(scenario.started_at).toLocaleString()}
                    </div>
                    <div>{scenario.total_events} events</div>
                    <div>{(scenario.duration_ms / 1000).toFixed(1)}s duration</div>
                  </div>
                </div>
                <div className="flex gap-2">
                  <button
                    onClick={() => replayScenario(scenario.name)}
                    className="flex-1 px-3 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 flex items-center justify-center gap-2 text-sm"
                  >
                    <Play className="h-3 w-3" />
                    Replay
                  </button>
                  <button
                    onClick={() => exportScenario(scenario.name)}
                    className="flex-1 px-3 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 flex items-center justify-center gap-2 text-sm"
                  >
                    <Download className="h-3 w-3" />
                    Export
                  </button>
                </div>
              </ModernCard>
            ))}
          </div>
        </Section>
      )}

      {/* Recorded Requests */}
      <Section
        title="Recorded Requests"
        subtitle={`${filteredRequests.length} requests`}
      >
        {/* Filters */}
        <div className="flex gap-4 mb-6">
          <div className="flex-1 relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-5 w-5 text-gray-400" />
            <input
              type="text"
              placeholder="Search by path or method..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100"
            />
          </div>
          <select
            value={protocolFilter}
            onChange={(e) => setProtocolFilter(e.target.value)}
            className="px-4 py-2 border border-gray-300 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100"
          >
            <option value="all">All Protocols</option>
            <option value="HTTP">HTTP</option>
            <option value="gRPC">gRPC</option>
            <option value="WebSocket">WebSocket</option>
            <option value="GraphQL">GraphQL</option>
          </select>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {/* Requests List */}
          <ModernCard>
            <div className="space-y-2 max-h-[600px] overflow-y-auto">
              {loading ? (
                <div className="text-center py-8">
                  <p className="text-gray-500 dark:text-gray-400">Loading requests...</p>
                </div>
              ) : filteredRequests.length === 0 ? (
                <div className="text-center py-8">
                  <p className="text-gray-500 dark:text-gray-400">No requests found</p>
                </div>
              ) : (
                filteredRequests.map(req => (
                  <div
                    key={req.id}
                    onClick={() => setSelectedRequest(req)}
                    className={`p-4 rounded-lg cursor-pointer border ${
                      selectedRequest?.id === req.id
                        ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                        : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2">
                        <ModernBadge size="sm">{req.method}</ModernBadge>
                        <span className="font-mono text-sm">{req.path}</span>
                      </div>
                      <ModernBadge
                        variant={
                          req.status_code >= 500 ? 'error' :
                          req.status_code >= 400 ? 'warning' : 'success'
                        }
                        size="sm"
                      >
                        {req.status_code}
                      </ModernBadge>
                    </div>
                    <div className="flex items-center gap-4 text-xs text-gray-500 dark:text-gray-400">
                      <span>{req.protocol}</span>
                      <span>{req.duration_ms.toFixed(2)}ms</span>
                      <span>{new Date(req.timestamp).toLocaleTimeString()}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </ModernCard>

          {/* Request Details */}
          <ModernCard>
            {!selectedRequest ? (
              <div className="text-center py-8">
                <p className="text-gray-500 dark:text-gray-400">
                  Select a request to view details
                </p>
              </div>
            ) : (
              <div className="space-y-4 max-h-[600px] overflow-y-auto">
                <div>
                  <h4 className="font-semibold text-gray-900 dark:text-gray-100 mb-2">Request Headers</h4>
                  <pre className="bg-gray-100 dark:bg-gray-800 p-3 rounded-lg overflow-x-auto text-xs font-mono">
                    {JSON.stringify(selectedRequest.request_headers, null, 2)}
                  </pre>
                </div>
                {selectedRequest.request_body && (
                  <div>
                    <h4 className="font-semibold text-gray-900 dark:text-gray-100 mb-2">Request Body</h4>
                    <pre className="bg-gray-100 dark:bg-gray-800 p-3 rounded-lg overflow-x-auto text-xs font-mono max-h-40">
                      {selectedRequest.request_body}
                    </pre>
                  </div>
                )}
                <div>
                  <h4 className="font-semibold text-gray-900 dark:text-gray-100 mb-2">Response Headers</h4>
                  <pre className="bg-gray-100 dark:bg-gray-800 p-3 rounded-lg overflow-x-auto text-xs font-mono">
                    {JSON.stringify(selectedRequest.response_headers, null, 2)}
                  </pre>
                </div>
                {selectedRequest.response_body && (
                  <div>
                    <h4 className="font-semibold text-gray-900 dark:text-gray-100 mb-2">Response Body</h4>
                    <pre className="bg-gray-100 dark:bg-gray-800 p-3 rounded-lg overflow-x-auto text-xs font-mono max-h-40">
                      {selectedRequest.response_body}
                    </pre>
                  </div>
                )}
              </div>
            )}
          </ModernCard>
        </div>
      </Section>
    </div>
  );
}
