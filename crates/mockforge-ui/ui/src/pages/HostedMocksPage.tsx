/**
 * Hosted Mocks Deployment Page
 *
 * Manage cloud-hosted mock service deployments
 */

import React, { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useHostedMockStream } from '@/hooks/useHostedMockStream';
import { useFlyRuntimeLogs } from '@/hooks/useFlyRuntimeLogs';
import { useRuntimeRequests } from '@/hooks/useRuntimeRequests';
import {
  useDeploymentCaptures,
  fetchCaptureResponse,
  type DeploymentCapture,
  type DeploymentCaptureResponse,
} from '@/hooks/useDeploymentCaptures';
import {
  useDeploymentTraces,
  fetchTraceSpans,
  type TraceSummary,
  type TraceSpan,
} from '@/hooks/useDeploymentTraces';
import {
  Box,
  Card,
  CardContent,
  CardActions,
  Grid,
  Typography,
  Button,
  Chip,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Alert,
  LinearProgress,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Tabs,
  Tab,
  List,
  ListItem,
  ListItemText,
  Divider,
  Tooltip,
  CircularProgress,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
} from '@mui/material';
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  Refresh as RefreshIcon,
  OpenInNew as OpenInNewIcon,
  CheckCircle as CheckCircleIcon,
  Error as ErrorIcon,
  Pending as PendingIcon,
  CloudUpload as CloudUploadIcon,
  Stop as StopIcon,
  Visibility as ViewIcon,
  Code as CodeIcon,
  Assessment as AssessmentIcon,
  Replay as ReplayIcon,
  Language as LanguageIcon,
  UploadFile as UploadFileIcon,
  PlayArrow as PlayArrowIcon,
} from '@mui/icons-material';

const FLY_REGIONS: { code: string; label: string }[] = [
  { code: 'iad', label: 'Ashburn, Virginia (US East)' },
  { code: 'ord', label: 'Chicago, Illinois (US Central)' },
  { code: 'sjc', label: 'San Jose, California (US West)' },
  { code: 'sea', label: 'Seattle, Washington (US West)' },
  { code: 'lhr', label: 'London, United Kingdom' },
  { code: 'fra', label: 'Frankfurt, Germany' },
  { code: 'ams', label: 'Amsterdam, Netherlands' },
  { code: 'cdg', label: 'Paris, France' },
  { code: 'nrt', label: 'Tokyo, Japan' },
  { code: 'sin', label: 'Singapore' },
  { code: 'syd', label: 'Sydney, Australia' },
  { code: 'gru', label: 'São Paulo, Brazil' },
];

type DeploymentProtocol =
  | 'http'
  | 'websocket'
  | 'graphql'
  | 'grpc'
  | 'smtp'
  | 'mqtt'
  | 'kafka'
  | 'amqp'
  | 'tcp';

interface Deployment {
  id: string;
  org_id: string;
  project_id?: string;
  name: string;
  slug: string;
  description?: string;
  status: 'pending' | 'deploying' | 'active' | 'stopped' | 'failed' | 'deleting';
  deployment_url?: string;
  openapi_spec_url?: string;
  region?: string;
  instance_type?: string;
  health_status: 'healthy' | 'unhealthy' | 'unknown';
  error_message?: string;
  enabled_protocols?: DeploymentProtocol[];
  created_at: string;
  updated_at: string;
}

interface DeploymentLog {
  id: string;
  level: 'info' | 'warning' | 'error' | 'debug';
  message: string;
  metadata: Record<string, any>;
  created_at: string;
}

interface DeploymentMetrics {
  requests: number;
  requests_2xx: number;
  requests_4xx: number;
  requests_5xx: number;
  egress_bytes: number;
  avg_response_time_ms: number;
  period_start: string;
}

interface ProjectOption {
  id: string;
  name: string;
  slug: string;
}

export const HostedMocksPage: React.FC = () => {
  const routerNavigate = useNavigate();

  const navigateToExplorer = useCallback((deployment: Deployment) => {
    if (!deployment.deployment_url) return;
    // Store deployment context for the ApiExplorerPage
    window.__mockforge_explorer_deployment = {
      id: deployment.id,
      name: deployment.name,
      deployment_url: deployment.deployment_url,
      status: deployment.status,
      openapi_spec_url: deployment.openapi_spec_url,
    };
    routerNavigate('/api-explorer');
  }, [routerNavigate]);
  const [deployments, setDeployments] = useState<Deployment[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [selectedDeployment, setSelectedDeployment] = useState<Deployment | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [detailsTab, setDetailsTab] = useState(0);
  const [logs, setLogs] = useState<DeploymentLog[]>([]);
  const [metrics, setMetrics] = useState<DeploymentMetrics | null>(null);
  const [logsLoading, setLogsLoading] = useState(false);
  const [metricsLoading, setMetricsLoading] = useState(false);

  // Create deployment form state
  const [formData, setFormData] = useState({
    name: '',
    slug: '',
    description: '',
    config_json: '{}',
    openapi_spec_url: '',
    region: 'iad',
    project_id: '',
  });

  // Optional protocols beyond HTTP. HTTP is implicit (always present);
  // WebSocket and GraphQL ride on the HTTP listener so toggling them is a
  // no-op — they're available unconditionally on Pro+ and don't show in the
  // picker. The picker covers protocols that need their own Fly service.
  type OptionalProtocol = 'grpc' | 'smtp' | 'mqtt' | 'kafka' | 'amqp' | 'tcp';
  const [enabledOptionalProtocols, setEnabledOptionalProtocols] = useState<OptionalProtocol[]>([]);

  const protocolOptions: Array<{ id: OptionalProtocol; label: string; minPlan: 'pro' | 'team'; hint: string }> = [
    { id: 'grpc', label: 'gRPC', minPlan: 'pro', hint: 'HTTP/2 service on port 50051' },
    { id: 'smtp', label: 'SMTP', minPlan: 'team', hint: 'Mock SMTP server on port 2525' },
    { id: 'mqtt', label: 'MQTT', minPlan: 'team', hint: 'MQTT broker on port 1883' },
    { id: 'kafka', label: 'Kafka', minPlan: 'team', hint: 'Kafka broker on port 9092' },
    { id: 'amqp', label: 'AMQP', minPlan: 'team', hint: 'AMQP broker on port 5672' },
    { id: 'tcp', label: 'Raw TCP', minPlan: 'team', hint: 'Custom TCP server on port 9999' },
  ];

  const toggleOptionalProtocol = (id: OptionalProtocol) => {
    setEnabledOptionalProtocols((prev) =>
      prev.includes(id) ? prev.filter((p) => p !== id) : [...prev, id],
    );
  };
  const [creating, setCreating] = useState(false);
  const [uploadingSpec, setUploadingSpec] = useState(false);
  const [redeployingId, setRedeployingId] = useState<string | null>(null);
  const [lifecycleId, setLifecycleId] = useState<string | null>(null);
  const [customDomain, setCustomDomain] = useState('');
  const [settingDomain, setSettingDomain] = useState(false);
  const [projects, setProjects] = useState<ProjectOption[]>([]);
  const [projectsLoadError, setProjectsLoadError] = useState<string | null>(null);

  // Real-time stream for the selected hosted mock deployment.
  // This goes direct to the deployment's `/__mockforge/ws` and gives
  // structured request log events (method/path/status/latency).
  const streamEnabled = detailsOpen && selectedDeployment?.status === 'active';
  const streamUrl = streamEnabled ? selectedDeployment?.deployment_url : undefined;
  const {
    connected: streamConnected,
    logs: streamLogs,
    metrics: streamMetrics,
  } = useHostedMockStream(streamUrl, { enabled: !!streamEnabled });

  // Fly runtime logs (container stdout/stderr) via the registry server's SSE
  // proxy. Complementary to the deployment WS stream above: works even when
  // the app's WS endpoint isn't reachable, captures startup logs, etc.
  const flyLogsEnabled = detailsOpen && !!selectedDeployment;
  const {
    entries: flyLogEntries,
    connected: flyLogsConnected,
    notConfigured: flyLogsNotConfigured,
    error: flyLogsError,
  } = useFlyRuntimeLogs(flyLogsEnabled ? selectedDeployment?.id : undefined, {
    enabled: flyLogsEnabled,
  });

  // Structured request log feed (#232). Polls the registry server's
  // /runtime-requests endpoint, which is populated by the in-container log
  // shipper. Surfaces in the new "Requests" tab below.
  const {
    rows: runtimeRequestRows,
    loading: runtimeRequestsLoading,
    error: runtimeRequestsError,
    refetch: refetchRuntimeRequests,
  } = useRuntimeRequests(detailsOpen ? selectedDeployment?.id : undefined, {
    enabled: detailsOpen && !!selectedDeployment,
  });

  // Filters for the Requests tab. Pure client-side — the rows are
  // already buffered by the hook so filtering doesn't trigger refetches.
  type RequestsStatusFilter = 'all' | '2xx' | '4xx' | '5xx';
  const [requestsStatusFilter, setRequestsStatusFilter] = useState<RequestsStatusFilter>('all');
  const [requestsPathFilter, setRequestsPathFilter] = useState('');

  const filteredRequestRows = React.useMemo(() => {
    const pathQuery = requestsPathFilter.trim().toLowerCase();
    return runtimeRequestRows.filter((row) => {
      if (requestsStatusFilter !== 'all') {
        const bucket = Math.floor(row.status / 100);
        if (requestsStatusFilter === '2xx' && bucket !== 2) return false;
        if (requestsStatusFilter === '4xx' && bucket !== 4) return false;
        if (requestsStatusFilter === '5xx' && bucket !== 5) return false;
      }
      if (pathQuery && !row.path.toLowerCase().includes(pathQuery)) {
        return false;
      }
      return true;
    });
  }, [runtimeRequestRows, requestsStatusFilter, requestsPathFilter]);

  // Recorder captures (#234) via the cloud proxy. The recorder library
  // stores full request/response pairs on the deployment's local SQLite;
  // the registry server proxies the read API so the data is reachable
  // from the admin UI without exposing the deployment URL to the browser.
  const {
    captures: recorderCaptures,
    loading: capturesLoading,
    error: capturesError,
    refetch: refetchCaptures,
  } = useDeploymentCaptures(detailsOpen ? selectedDeployment?.id : undefined, {
    enabled: detailsOpen && !!selectedDeployment,
  });

  // Filters for the Captures tab. Mirrors the Requests tab UX so a user
  // who learns one knows the other. Status filter buckets HTTP responses
  // (recorder also captures non-HTTP protocols where status is null —
  // those count as 'all' but are excluded from the 2xx/4xx/5xx buckets).
  const [capturesStatusFilter, setCapturesStatusFilter] =
    useState<RequestsStatusFilter>('all');
  const [capturesPathFilter, setCapturesPathFilter] = useState('');

  const filteredCaptures = React.useMemo(() => {
    const pathQuery = capturesPathFilter.trim().toLowerCase();
    return recorderCaptures.filter((capture) => {
      if (capturesStatusFilter !== 'all') {
        if (capture.status_code == null) return false;
        const bucket = Math.floor(capture.status_code / 100);
        if (capturesStatusFilter === '2xx' && bucket !== 2) return false;
        if (capturesStatusFilter === '4xx' && bucket !== 4) return false;
        if (capturesStatusFilter === '5xx' && bucket !== 5) return false;
      }
      if (pathQuery && !capture.path.toLowerCase().includes(pathQuery)) {
        return false;
      }
      return true;
    });
  }, [recorderCaptures, capturesStatusFilter, capturesPathFilter]);
  const [selectedCapture, setSelectedCapture] = useState<DeploymentCapture | null>(null);
  const [selectedCaptureResponse, setSelectedCaptureResponse] =
    useState<DeploymentCaptureResponse | null>(null);
  const [captureResponseLoading, setCaptureResponseLoading] = useState(false);
  const [replayResult, setReplayResult] = useState<unknown | null>(null);
  const [replayLoading, setReplayLoading] = useState(false);
  const [replayError, setReplayError] = useState<string | null>(null);

  // OTLP traces (#233). Same proxy-pattern as captures: the deployment's
  // mockforge-cli sends spans via OTLP/HTTP-JSON to the registry; we
  // surface them here so the user can drill into a single request's
  // span tree without leaving the admin UI.
  const {
    traces: deploymentTraces,
    loading: tracesLoading,
    error: tracesError,
    refetch: refetchTraces,
  } = useDeploymentTraces(detailsOpen ? selectedDeployment?.id : undefined, {
    enabled: detailsOpen && !!selectedDeployment,
  });
  const [selectedTrace, setSelectedTrace] = useState<TraceSummary | null>(null);
  const [selectedTraceSpans, setSelectedTraceSpans] = useState<TraceSpan[]>([]);
  const [traceSpansLoading, setTraceSpansLoading] = useState(false);

  const openTrace = useCallback(
    async (trace: TraceSummary) => {
      setSelectedTrace(trace);
      setSelectedTraceSpans([]);
      if (!selectedDeployment) return;
      setTraceSpansLoading(true);
      try {
        const spans = await fetchTraceSpans(selectedDeployment.id, trace.trace_id);
        setSelectedTraceSpans(spans);
      } finally {
        setTraceSpansLoading(false);
      }
    },
    [selectedDeployment],
  );

  const openCapture = useCallback(
    async (capture: DeploymentCapture) => {
      setSelectedCapture(capture);
      setSelectedCaptureResponse(null);
      setReplayResult(null);
      setReplayError(null);
      if (!selectedDeployment) return;
      setCaptureResponseLoading(true);
      try {
        const resp = await fetchCaptureResponse(selectedDeployment.id, capture.id);
        setSelectedCaptureResponse(resp);
      } finally {
        setCaptureResponseLoading(false);
      }
    },
    [selectedDeployment],
  );

  const replaySelectedCapture = useCallback(async () => {
    if (!selectedDeployment || !selectedCapture) return;
    setReplayLoading(true);
    setReplayError(null);
    setReplayResult(null);
    try {
      const token = localStorage.getItem('auth_token');
      const url = `/api/v1/hosted-mocks/${encodeURIComponent(selectedDeployment.id)}/captures/${encodeURIComponent(selectedCapture.id)}/replay`;
      const resp = await fetch(url, {
        method: 'POST',
        headers: { Authorization: `Bearer ${token ?? ''}` },
      });
      if (!resp.ok) {
        throw new Error(`HTTP ${resp.status}`);
      }
      const contentType = resp.headers.get('content-type') ?? '';
      const data = contentType.includes('application/json') ? await resp.json() : await resp.text();
      setReplayResult(data);
    } catch (err) {
      setReplayError(err instanceof Error ? err.message : 'Replay failed');
    } finally {
      setReplayLoading(false);
    }
  }, [selectedDeployment, selectedCapture]);

  useEffect(() => {
    loadDeployments();
    loadProjects();
  }, []);

  const loadProjects = async () => {
    setProjectsLoadError(null);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) return;

      const response = await fetch('/api/v1/projects', {
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        // Non-fatal: project_id is optional
        setProjectsLoadError(`Failed to load projects (HTTP ${response.status})`);
        return;
      }

      const data = await response.json();
      setProjects(
        Array.isArray(data)
          ? data.map((p: { id: string; name: string; slug: string }) => ({
              id: p.id,
              name: p.name,
              slug: p.slug,
            }))
          : [],
      );
    } catch {
      setProjectsLoadError('Failed to load projects');
    }
  };

  // Auto-refresh deployments while any are pending/deploying.
  useEffect(() => {
    const hasInFlight = Array.isArray(deployments) && deployments.some(
      d => d.status === 'pending' || d.status === 'deploying'
    );
    if (!hasInFlight) return;
    const id = setInterval(() => {
      loadDeployments();
    }, 5000);
    return () => clearInterval(id);
  }, [deployments]);

  const loadDeployments = async () => {
    setLoading(true);
    setError(null);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated');
      }

      const response = await fetch('/api/v1/hosted-mocks', {
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        throw new Error('Failed to load deployments');
      }

      const data = await response.json();
      setDeployments(Array.isArray(data) ? data : []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load deployments');
    } finally {
      setLoading(false);
    }
  };

  const handleCreateDeployment = async () => {
    setCreating(true);
    setError(null);

    try {
      // Validate form
      if (!formData.name || !formData.config_json) {
        throw new Error('Name and config are required');
      }

      // Validate JSON
      let configJson;
      try {
        configJson = JSON.parse(formData.config_json);
      } catch {
        throw new Error('Invalid JSON in config field');
      }

      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated');
      }

      // HTTP is always implicit. The picker only adds protocols that need
      // their own Fly service entry; the backend rejects with a clear error
      // if the org's plan can't accommodate the request.
      const enabledProtocols = ['http', ...enabledOptionalProtocols];

      const request = {
        name: formData.name,
        slug: formData.slug || undefined,
        description: formData.description || undefined,
        config_json: configJson,
        openapi_spec_url: formData.openapi_spec_url || undefined,
        region: formData.region || undefined,
        project_id: formData.project_id || undefined,
        enabled_protocols: enabledProtocols,
      };

      const response = await fetch('/api/v1/hosted-mocks', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
        // `errorData.error` may be a string, an object (e.g. validation
        // details), or missing entirely. Normalize to a human-readable
        // string so the toast doesn't render "[object Object]".
        const raw = errorData?.error ?? errorData?.message;
        const message =
          typeof raw === 'string'
            ? raw
            : raw
              ? JSON.stringify(raw)
              : `Failed to create deployment (HTTP ${response.status})`;
        throw new Error(message);
      }

      // Success
      setCreateModalOpen(false);
      setFormData({
        name: '',
        slug: '',
        description: '',
        config_json: '{}',
        openapi_spec_url: '',
        region: 'iad',
        project_id: '',
      });
      setEnabledOptionalProtocols([]);
      loadDeployments();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create deployment');
    } finally {
      setCreating(false);
    }
  };

  const handleDeleteDeployment = async (id: string) => {
    if (!confirm('Are you sure you want to delete this deployment?')) {
      return;
    }

    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated');
      }

      const response = await fetch(`/api/v1/hosted-mocks/${id}`, {
        method: 'DELETE',
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        throw new Error('Failed to delete deployment');
      }

      loadDeployments();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete deployment');
    }
  };

  const handleUploadSpec = async (file: File) => {
    setUploadingSpec(true);
    setError(null);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated');
      }

      const body = new FormData();
      body.append('file', file);

      const response = await fetch('/api/v1/hosted-mocks/specs/upload', {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${token}`,
        },
        body,
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Upload failed' }));
        const raw = errorData?.error ?? errorData?.message;
        const message =
          typeof raw === 'string'
            ? raw
            : raw
              ? JSON.stringify(raw)
              : `Spec upload failed (HTTP ${response.status})`;
        throw new Error(message);
      }

      const data = await response.json();
      if (typeof data?.url === 'string') {
        setFormData((prev) => ({ ...prev, openapi_spec_url: data.url }));
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to upload spec');
    } finally {
      setUploadingSpec(false);
    }
  };

  const handleLifecycleAction = async (
    id: string,
    action: 'stop' | 'start',
  ) => {
    const label = action === 'stop' ? 'Stop' : 'Start';
    const confirmMsg =
      action === 'stop'
        ? 'Stop this mock service? Requests will be refused until it is started again.'
        : 'Start this mock service?';
    if (!confirm(confirmMsg)) return;

    setLifecycleId(id);
    setError(null);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated');
      }

      const response = await fetch(`/api/v1/hosted-mocks/${id}/${action}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: `${label} failed` }));
        const raw = errorData?.error ?? errorData?.message;
        const message =
          typeof raw === 'string'
            ? raw
            : raw
              ? JSON.stringify(raw)
              : `${label} failed (HTTP ${response.status})`;
        throw new Error(message);
      }

      loadDeployments();
    } catch (err) {
      setError(err instanceof Error ? err.message : `Failed to ${action} deployment`);
    } finally {
      setLifecycleId(null);
    }
  };

  const handleRedeployDeployment = async (id: string) => {
    if (!confirm('Redeploy this mock service? Active traffic may be briefly interrupted.')) {
      return;
    }
    setRedeployingId(id);
    setError(null);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated');
      }

      const response = await fetch(`/api/v1/hosted-mocks/${id}/redeploy`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({}),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Redeploy failed' }));
        const raw = errorData?.error ?? errorData?.message;
        const message =
          typeof raw === 'string'
            ? raw
            : raw
              ? JSON.stringify(raw)
              : `Redeploy failed (HTTP ${response.status})`;
        throw new Error(message);
      }

      loadDeployments();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to redeploy');
    } finally {
      setRedeployingId(null);
    }
  };

  const handleSetDomain = async () => {
    if (!selectedDeployment || !customDomain.trim()) return;
    setSettingDomain(true);
    setError(null);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated');
      }

      const response = await fetch(
        `/api/v1/hosted-mocks/${selectedDeployment.id}/set-domain`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${token}`,
          },
          body: JSON.stringify({ domain: customDomain.trim() }),
        },
      );

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Set domain failed' }));
        const raw = errorData?.error ?? errorData?.message;
        const message =
          typeof raw === 'string'
            ? raw
            : raw
              ? JSON.stringify(raw)
              : `Set domain failed (HTTP ${response.status})`;
        throw new Error(message);
      }

      const data = await response.json();
      if (typeof data?.deployment_url === 'string') {
        setSelectedDeployment({ ...selectedDeployment, deployment_url: data.deployment_url });
      }
      setCustomDomain('');
      loadDeployments();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to set custom domain');
    } finally {
      setSettingDomain(false);
    }
  };

  const handleViewDetails = async (deployment: Deployment) => {
    setSelectedDeployment(deployment);
    setDetailsOpen(true);
    setDetailsTab(0);
    setLogs([]);
    setMetrics(null);
    setCustomDomain('');

    // Load logs and metrics
    await Promise.all([
      loadDeploymentLogs(deployment.id),
      loadDeploymentMetrics(deployment.id),
    ]);
  };

  const loadDeploymentLogs = async (id: string) => {
    setLogsLoading(true);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        return;
      }

      const response = await fetch(`/api/v1/hosted-mocks/${id}/logs`, {
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (response.ok) {
        const data = await response.json();
        setLogs(data || []);
      }
    } catch (err) {
      console.error('Failed to load logs:', err);
    } finally {
      setLogsLoading(false);
    }
  };

  const loadDeploymentMetrics = async (id: string) => {
    setMetricsLoading(true);
    try {
      const token = localStorage.getItem('auth_token');
      if (!token) {
        return;
      }

      const response = await fetch(`/api/v1/hosted-mocks/${id}/metrics`, {
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

      if (response.ok) {
        const data = await response.json();
        setMetrics(data);
      }
    } catch (err) {
      console.error('Failed to load metrics:', err);
    } finally {
      setMetricsLoading(false);
    }
  };

  const getStatusColor = (status: Deployment['status']) => {
    switch (status) {
      case 'active':
        return 'success';
      case 'failed':
        return 'error';
      case 'deploying':
      case 'pending':
        return 'warning';
      case 'stopped':
        return 'default';
      default:
        return 'default';
    }
  };

  const getStatusIcon = (status: Deployment['status']) => {
    switch (status) {
      case 'active':
        return <CheckCircleIcon />;
      case 'failed':
        return <ErrorIcon />;
      case 'deploying':
      case 'pending':
        return <PendingIcon />;
      default:
        return <StopIcon />;
    }
  };

  const getHealthColor = (health: Deployment['health_status']) => {
    switch (health) {
      case 'healthy':
        return 'success';
      case 'unhealthy':
        return 'error';
      default:
        return 'default';
    }
  };

  return (
    <Box sx={{ p: 3 }}>
      {/* Header */}
      <Box sx={{ mb: 4, display: 'flex', justifyContent: 'space-between', alignItems: 'start' }}>
        <Box>
          <Typography variant="h4" gutterBottom>
            Hosted Mocks
          </Typography>
          <Typography variant="body1" color="text.secondary">
            Deploy and manage cloud-hosted mock services
          </Typography>
        </Box>
        <Box sx={{ display: 'flex', gap: 2 }}>
          <Button
            variant="outlined"
            startIcon={<RefreshIcon />}
            onClick={loadDeployments}
            disabled={loading}
          >
            Refresh
          </Button>
          <Button
            variant="contained"
            startIcon={<AddIcon />}
            onClick={() => setCreateModalOpen(true)}
          >
            Deploy Mock
          </Button>
        </Box>
      </Box>

      {error && (
        <Alert severity="error" sx={{ mb: 3 }} onClose={() => setError(null)}>
          {error}
        </Alert>
      )}

      {loading && <LinearProgress sx={{ mb: 3 }} />}

      {/* Deployments Table */}
      <Card>
        <TableContainer>
          <Table>
            <TableHead>
              <TableRow>
                <TableCell>Name</TableCell>
                <TableCell>Status</TableCell>
                <TableCell>Health</TableCell>
                <TableCell>URL</TableCell>
                <TableCell>Created</TableCell>
                <TableCell align="right">Actions</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {deployments.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 4 }}>
                    <Typography variant="body2" color="text.secondary">
                      No deployments yet. Create your first deployment to get started.
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                deployments.map((deployment) => (
                  <TableRow key={deployment.id}>
                    <TableCell>
                      <Box>
                        <Typography variant="subtitle2">{deployment.name}</Typography>
                        {deployment.description && (
                          <Typography variant="caption" color="text.secondary">
                            {deployment.description}
                          </Typography>
                        )}
                      </Box>
                    </TableCell>
                    <TableCell>
                      <Chip
                        icon={getStatusIcon(deployment.status)}
                        label={deployment.status}
                        color={getStatusColor(deployment.status) as any}
                        size="small"
                      />
                    </TableCell>
                    <TableCell>
                      <Chip
                        label={deployment.health_status}
                        color={getHealthColor(deployment.health_status) as any}
                        size="small"
                      />
                    </TableCell>
                    <TableCell>
                      {deployment.deployment_url ? (
                        <Button
                          size="small"
                          startIcon={<CodeIcon />}
                          onClick={() => navigateToExplorer(deployment)}
                        >
                          Explore
                        </Button>
                      ) : (
                        <Typography variant="caption" color="text.secondary">
                          Not available
                        </Typography>
                      )}
                    </TableCell>
                    <TableCell>
                      <Typography variant="caption">
                        {new Date(deployment.created_at).toLocaleDateString()}
                      </Typography>
                    </TableCell>
                    <TableCell align="right">
                      <Tooltip title="View Details">
                        <IconButton
                          size="small"
                          onClick={() => handleViewDetails(deployment)}
                        >
                          <ViewIcon />
                        </IconButton>
                      </Tooltip>
                      {(deployment.status === 'active' || deployment.status === 'failed') && (
                        <Tooltip title="Redeploy">
                          <span>
                            <IconButton
                              size="small"
                              onClick={() => handleRedeployDeployment(deployment.id)}
                              disabled={redeployingId === deployment.id}
                            >
                              {redeployingId === deployment.id ? (
                                <CircularProgress size={18} />
                              ) : (
                                <ReplayIcon />
                              )}
                            </IconButton>
                          </span>
                        </Tooltip>
                      )}
                      {deployment.status === 'active' && (
                        <Tooltip title="Stop">
                          <span>
                            <IconButton
                              size="small"
                              onClick={() => handleLifecycleAction(deployment.id, 'stop')}
                              disabled={lifecycleId === deployment.id}
                            >
                              {lifecycleId === deployment.id ? (
                                <CircularProgress size={18} />
                              ) : (
                                <StopIcon />
                              )}
                            </IconButton>
                          </span>
                        </Tooltip>
                      )}
                      {deployment.status === 'stopped' && (
                        <Tooltip title="Start">
                          <span>
                            <IconButton
                              size="small"
                              color="success"
                              onClick={() => handleLifecycleAction(deployment.id, 'start')}
                              disabled={lifecycleId === deployment.id}
                            >
                              {lifecycleId === deployment.id ? (
                                <CircularProgress size={18} />
                              ) : (
                                <PlayArrowIcon />
                              )}
                            </IconButton>
                          </span>
                        </Tooltip>
                      )}
                      <Tooltip title="Delete">
                        <IconButton
                          size="small"
                          color="error"
                          onClick={() => handleDeleteDeployment(deployment.id)}
                        >
                          <DeleteIcon />
                        </IconButton>
                      </Tooltip>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Card>

      {/* Create Deployment Modal */}
      <Dialog open={createModalOpen} onClose={() => setCreateModalOpen(false)} maxWidth="md" fullWidth>
        <DialogTitle>Deploy New Mock Service</DialogTitle>
        <DialogContent>
          {error && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {error}
            </Alert>
          )}
          {creating && <LinearProgress sx={{ mb: 2 }} />}

          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mt: 1 }}>
            <TextField
              label="Name"
              required
              fullWidth
              value={formData.name}
              onChange={(e) => {
                setFormData({ ...formData, name: e.target.value });
                // Auto-generate slug
                if (!formData.slug) {
                  const slug = e.target.value
                    .toLowerCase()
                    .replace(/[^a-z0-9]+/g, '-')
                    .replace(/^-+|-+$/g, '');
                  setFormData((prev) => ({ ...prev, slug }));
                }
              }}
              placeholder="My Mock Service"
            />

            <TextField
              label="Slug"
              fullWidth
              value={formData.slug}
              onChange={(e) => setFormData({ ...formData, slug: e.target.value })}
              placeholder="my-mock-service"
              helperText="URL-friendly identifier (auto-generated from name)"
            />

            <TextField
              label="Description"
              fullWidth
              multiline
              rows={2}
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              placeholder="Description of the mock service"
            />

            <FormControl fullWidth>
              <InputLabel id="hosted-mock-project-label">Project (optional)</InputLabel>
              <Select
                labelId="hosted-mock-project-label"
                label="Project (optional)"
                value={formData.project_id}
                onChange={(e) =>
                  setFormData({ ...formData, project_id: e.target.value as string })
                }
                displayEmpty
              >
                <MenuItem value="">
                  <em>No project</em>
                </MenuItem>
                {projects.map((p) => (
                  <MenuItem key={p.id} value={p.id}>
                    {p.name}
                    {p.slug ? ` (${p.slug})` : ''}
                  </MenuItem>
                ))}
              </Select>
              {projectsLoadError && (
                <Typography variant="caption" color="warning.main" sx={{ mt: 0.5 }}>
                  {projectsLoadError}
                </Typography>
              )}
            </FormControl>

            <FormControl fullWidth>
              <InputLabel id="hosted-mock-region-label">Region</InputLabel>
              <Select
                labelId="hosted-mock-region-label"
                label="Region"
                value={formData.region}
                onChange={(e) => setFormData({ ...formData, region: e.target.value as string })}
              >
                {FLY_REGIONS.map((r) => (
                  <MenuItem key={r.code} value={r.code}>
                    {r.label} ({r.code})
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

            <Box sx={{ display: 'flex', gap: 1, alignItems: 'flex-start' }}>
              <TextField
                label="OpenAPI Spec URL (optional)"
                fullWidth
                value={formData.openapi_spec_url}
                onChange={(e) => setFormData({ ...formData, openapi_spec_url: e.target.value })}
                placeholder="https://example.com/openapi.json"
                helperText="Paste a URL or upload a JSON/YAML spec"
              />
              <Button
                variant="outlined"
                component="label"
                startIcon={uploadingSpec ? <CircularProgress size={16} /> : <UploadFileIcon />}
                disabled={uploadingSpec}
                sx={{ whiteSpace: 'nowrap', mt: 1 }}
              >
                {uploadingSpec ? 'Uploading…' : 'Upload'}
                <input
                  type="file"
                  accept=".json,.yaml,.yml,application/json,application/yaml,text/yaml"
                  hidden
                  onChange={(e) => {
                    const file = e.target.files?.[0];
                    if (file) {
                      handleUploadSpec(file);
                    }
                    // Reset so selecting the same file again still fires onChange
                    e.target.value = '';
                  }}
                />
              </Button>
            </Box>

            <Box sx={{ mb: 2 }}>
              <Typography variant="subtitle2" sx={{ mb: 0.5 }}>
                Protocols
              </Typography>
              <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
                HTTP, WebSocket, and GraphQL are always available on port 3000. Enable additional
                protocols below to expose them on dedicated Fly ports. Plan-gated — the backend
                will reject the request if your plan doesn't cover what you select.
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
                {protocolOptions.map((opt) => {
                  const checked = enabledOptionalProtocols.includes(opt.id);
                  return (
                    <Chip
                      key={opt.id}
                      label={
                        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                          <span>{opt.label}</span>
                          <Box
                            component="span"
                            sx={{
                              fontSize: 10,
                              px: 0.75,
                              py: 0.1,
                              borderRadius: 1,
                              bgcolor: opt.minPlan === 'team' ? 'secondary.main' : 'primary.main',
                              color: 'common.white',
                              textTransform: 'uppercase',
                            }}
                          >
                            {opt.minPlan}
                          </Box>
                        </Box>
                      }
                      onClick={() => toggleOptionalProtocol(opt.id)}
                      color={checked ? 'primary' : 'default'}
                      variant={checked ? 'filled' : 'outlined'}
                      title={opt.hint}
                    />
                  );
                })}
              </Box>
            </Box>

            <TextField
              label="Configuration (JSON)"
              required
              fullWidth
              multiline
              rows={8}
              value={formData.config_json}
              onChange={(e) => setFormData({ ...formData, config_json: e.target.value })}
              placeholder='{"services": [], "plugins": []}'
              helperText="MockForge configuration in JSON format"
            />
          </Box>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setCreateModalOpen(false)} disabled={creating}>
            Cancel
          </Button>
          <Button
            variant="contained"
            onClick={handleCreateDeployment}
            disabled={creating || !formData.name || !formData.config_json}
            startIcon={<CloudUploadIcon />}
          >
            {creating ? 'Deploying...' : 'Deploy'}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Deployment Details Dialog */}
      <Dialog
        open={detailsOpen}
        onClose={() => setDetailsOpen(false)}
        maxWidth="lg"
        fullWidth
      >
        {selectedDeployment && (
          <>
            <DialogTitle>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <Box>
                  <Typography variant="h6">{selectedDeployment.name}</Typography>
                  <Typography variant="caption" color="text.secondary">
                    {selectedDeployment.slug}
                  </Typography>
                </Box>
                <Box sx={{ display: 'flex', gap: 1 }}>
                  <Chip
                    icon={getStatusIcon(selectedDeployment.status)}
                    label={selectedDeployment.status}
                    color={getStatusColor(selectedDeployment.status) as any}
                  />
                  <Chip
                    label={selectedDeployment.health_status}
                    color={getHealthColor(selectedDeployment.health_status) as any}
                  />
                </Box>
              </Box>
            </DialogTitle>

            <DialogContent>
              <Tabs value={detailsTab} onChange={(_, v) => setDetailsTab(v)} sx={{ mb: 3 }}>
                <Tab icon={<CodeIcon />} label="Overview" />
                <Tab icon={<ViewIcon />} label="Events" />
                <Tab icon={<ViewIcon />} label="Logs" />
                <Tab icon={<ViewIcon />} label="Requests" />
                <Tab icon={<ViewIcon />} label="Captures" />
                <Tab icon={<ViewIcon />} label="Traces" />
                <Tab icon={<AssessmentIcon />} label="Metrics" />
              </Tabs>

              {detailsTab === 0 && (
                <Box>
                  {selectedDeployment.description && (
                    <Typography variant="body1" paragraph>
                      {selectedDeployment.description}
                    </Typography>
                  )}

                  {selectedDeployment.error_message && (
                    <Alert severity="error" sx={{ mb: 2 }}>
                      {selectedDeployment.error_message}
                    </Alert>
                  )}

                  <Grid container spacing={2}>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        Deployment URL
                      </Typography>
                      <Typography variant="body2">
                        {selectedDeployment.deployment_url ? (
                          <Button
                            size="small"
                            startIcon={<OpenInNewIcon />}
                            href={selectedDeployment.deployment_url}
                            target="_blank"
                            rel="noopener noreferrer"
                          >
                            {selectedDeployment.deployment_url}
                          </Button>
                        ) : (
                          'Not available'
                        )}
                      </Typography>
                    </Grid>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        OpenAPI Spec
                      </Typography>
                      <Typography variant="body2">
                        {selectedDeployment.openapi_spec_url ? (
                          <Button
                            size="small"
                            startIcon={<OpenInNewIcon />}
                            href={selectedDeployment.openapi_spec_url}
                            target="_blank"
                            rel="noopener noreferrer"
                          >
                            View spec
                          </Button>
                        ) : (
                          'Not provided'
                        )}
                      </Typography>
                    </Grid>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        Region
                      </Typography>
                      <Typography variant="body2">
                        {selectedDeployment.region || 'Unknown'}
                      </Typography>
                    </Grid>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        Created
                      </Typography>
                      <Typography variant="body2">
                        {new Date(selectedDeployment.created_at).toLocaleString()}
                      </Typography>
                    </Grid>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        Last Updated
                      </Typography>
                      <Typography variant="body2">
                        {new Date(selectedDeployment.updated_at).toLocaleString()}
                      </Typography>
                    </Grid>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        ID
                      </Typography>
                      <Typography variant="body2" sx={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>
                        {selectedDeployment.id}
                      </Typography>
                    </Grid>
                  </Grid>

                  <Divider sx={{ my: 3 }} />

                  <Box>
                    <Typography variant="subtitle2" gutterBottom>
                      Custom Domain
                    </Typography>
                    <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
                      Map this deployment to <strong>{selectedDeployment.slug}.&lt;your-domain&gt;</strong>.
                      The registry wildcard TLS cert terminates traffic and proxies to the deployment.
                    </Typography>
                    <Box sx={{ display: 'flex', gap: 1, alignItems: 'flex-start' }}>
                      <TextField
                        size="small"
                        fullWidth
                        placeholder="mocks.example.com"
                        value={customDomain}
                        onChange={(e) => setCustomDomain(e.target.value)}
                        disabled={settingDomain}
                      />
                      <Button
                        variant="contained"
                        startIcon={settingDomain ? <CircularProgress size={16} /> : <LanguageIcon />}
                        onClick={handleSetDomain}
                        disabled={settingDomain || !customDomain.trim()}
                        sx={{ whiteSpace: 'nowrap' }}
                      >
                        {settingDomain ? 'Applying…' : 'Set Domain'}
                      </Button>
                    </Box>
                  </Box>
                </Box>
              )}

              {/* Events tab: deployment lifecycle history (created → deploying → active / failed). */}
              {detailsTab === 1 && (
                <Box>
                  <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                    Deployment lifecycle events. For runtime container output, see the Logs tab.
                  </Typography>
                  {logsLoading ? (
                    <Box sx={{ display: 'flex', justifyContent: 'center', p: 3 }}>
                      <CircularProgress />
                    </Box>
                  ) : logs.length === 0 ? (
                    <Alert severity="info">No lifecycle events recorded yet.</Alert>
                  ) : (
                    <List>
                      {logs.map((log, index) => (
                        <React.Fragment key={log.id}>
                          <ListItem>
                            <ListItemText
                              primary={
                                <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                                  <Chip
                                    label={log.level}
                                    size="small"
                                    color={
                                      log.level === 'error'
                                        ? 'error'
                                        : log.level === 'warning'
                                        ? 'warning'
                                        : 'default'
                                    }
                                  />
                                  <Typography variant="body2">{log.message}</Typography>
                                </Box>
                              }
                              secondary={new Date(log.created_at).toLocaleString()}
                            />
                          </ListItem>
                          {index < logs.length - 1 && <Divider />}
                        </React.Fragment>
                      ))}
                    </List>
                  )}
                </Box>
              )}

              {/* Logs tab: runtime container logs (Fly SSE) + live request stream from the deployment WS. */}
              {detailsTab === 2 && (
                <Box>
                  {flyLogsNotConfigured && (
                    <Alert severity="warning" sx={{ mb: 2 }}>
                      Fly runtime log streaming isn't configured on this MockForge Cloud instance
                      (FLYIO_API_TOKEN unset). Container stdout/stderr won't appear here, but live
                      request events from the deployment WebSocket will.
                    </Alert>
                  )}
                  {flyLogsError && (
                    <Alert severity="error" sx={{ mb: 2 }}>
                      Fly logs error: {flyLogsError}
                    </Alert>
                  )}
                  {flyLogsConnected && !flyLogsNotConfigured && (
                    <Alert severity="success" sx={{ mb: 2 }}>
                      Streaming runtime logs from Fly
                    </Alert>
                  )}

                  {flyLogEntries.length > 0 && (
                    <Box sx={{ mb: 3 }}>
                      <Typography variant="subtitle2" sx={{ mb: 1 }}>
                        Container logs ({flyLogEntries.length})
                      </Typography>
                      <Box
                        sx={{
                          maxHeight: 320,
                          overflow: 'auto',
                          fontFamily: 'monospace',
                          fontSize: 12,
                          bgcolor: 'background.default',
                          border: 1,
                          borderColor: 'divider',
                          borderRadius: 1,
                          p: 1.5,
                        }}
                      >
                        {flyLogEntries.map((entry, index) => (
                          <Box
                            key={`fly-${index}-${entry.timestamp}`}
                            sx={{ display: 'flex', gap: 1, py: 0.25 }}
                          >
                            <Typography
                              component="span"
                              variant="caption"
                              color="text.secondary"
                              sx={{ minWidth: 160 }}
                            >
                              {new Date(entry.timestamp).toLocaleTimeString()}
                            </Typography>
                            <Typography
                              component="span"
                              variant="caption"
                              sx={{
                                minWidth: 50,
                                color:
                                  entry.level === 'error'
                                    ? 'error.main'
                                    : entry.level === 'warning'
                                    ? 'warning.main'
                                    : 'text.secondary',
                              }}
                            >
                              {entry.level}
                            </Typography>
                            <Typography
                              component="span"
                              variant="body2"
                              sx={{ flex: 1, whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}
                            >
                              {entry.message}
                            </Typography>
                          </Box>
                        ))}
                      </Box>
                    </Box>
                  )}

                  {streamConnected && (
                    <Alert severity="success" sx={{ mb: 2 }}>
                      Live streaming connected — new requests will appear automatically
                    </Alert>
                  )}
                  {streamLogs.length > 0 && (
                    <Box>
                      <Typography variant="subtitle2" sx={{ mb: 1 }}>
                        Live requests
                      </Typography>
                      <List dense>
                        {streamLogs.slice(0, 50).map((entry, index) => (
                          <React.Fragment key={`stream-${entry.request_id || index}`}>
                            <ListItem>
                              <ListItemText
                                primary={
                                  <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                                    <Chip
                                      label={entry.method}
                                      size="small"
                                      color="primary"
                                      variant="outlined"
                                    />
                                    <Typography variant="body2" sx={{ fontFamily: 'monospace' }}>
                                      {entry.path}
                                    </Typography>
                                    <Chip
                                      label={entry.status}
                                      size="small"
                                      color={entry.status >= 500 ? 'error' : entry.status >= 400 ? 'warning' : 'success'}
                                    />
                                    <Typography variant="caption" color="text.secondary">
                                      {entry.latency_ms}ms
                                    </Typography>
                                  </Box>
                                }
                                secondary={new Date(entry.timestamp).toLocaleString()}
                              />
                            </ListItem>
                            {index < Math.min(streamLogs.length, 50) - 1 && <Divider />}
                          </React.Fragment>
                        ))}
                      </List>
                    </Box>
                  )}

                  {flyLogEntries.length === 0 && streamLogs.length === 0 && (
                    <Alert severity="info">
                      Waiting for log output. Container logs appear here as Fly emits them; request
                      logs appear when the deployment is reachable and traffic arrives.
                    </Alert>
                  )}
                </Box>
              )}

              {/* Requests tab: structured request log feed from the in-container shipper (#232). */}
              {detailsTab === 3 && (
                <Box>
                  <Box sx={{ display: 'flex', alignItems: 'center', mb: 2, gap: 1 }}>
                    <Typography variant="body2" color="text.secondary" sx={{ flex: 1 }}>
                      Live request feed shipped from the deployed container. Polls every 4
                      seconds.
                    </Typography>
                    <Button
                      size="small"
                      variant="outlined"
                      onClick={refetchRuntimeRequests}
                      disabled={runtimeRequestsLoading}
                    >
                      Refresh
                    </Button>
                  </Box>

                  {/* Filter toolbar: status bucket chips + path substring search. */}
                  <Box
                    sx={{
                      display: 'flex',
                      alignItems: 'center',
                      flexWrap: 'wrap',
                      gap: 1,
                      mb: 2,
                    }}
                  >
                    {(['all', '2xx', '4xx', '5xx'] as const).map((bucket) => (
                      <Chip
                        key={bucket}
                        label={bucket === 'all' ? 'All' : bucket}
                        size="small"
                        clickable
                        onClick={() => setRequestsStatusFilter(bucket)}
                        color={
                          requestsStatusFilter !== bucket
                            ? 'default'
                            : bucket === '5xx'
                              ? 'error'
                              : bucket === '4xx'
                                ? 'warning'
                                : bucket === '2xx'
                                  ? 'success'
                                  : 'primary'
                        }
                        variant={requestsStatusFilter === bucket ? 'filled' : 'outlined'}
                      />
                    ))}
                    <TextField
                      size="small"
                      placeholder="Filter by path…"
                      value={requestsPathFilter}
                      onChange={(e) => setRequestsPathFilter(e.target.value)}
                      sx={{ minWidth: 240, ml: 'auto' }}
                    />
                  </Box>

                  {runtimeRequestsError && (
                    <Alert severity="warning" sx={{ mb: 2 }}>
                      Request feed error: {runtimeRequestsError}. The cloud may not have
                      MOCKFORGE_LOG_INGEST_BASE_URL configured, or the container hasn't
                      shipped any batches yet.
                    </Alert>
                  )}

                  {(requestsStatusFilter !== 'all' || requestsPathFilter.trim()) &&
                    runtimeRequestRows.length > 0 && (
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{ display: 'block', mb: 1 }}
                      >
                        Showing {filteredRequestRows.length} of {runtimeRequestRows.length}{' '}
                        captured requests.
                      </Typography>
                    )}

                  {runtimeRequestRows.length === 0 ? (
                    <Alert severity="info">
                      Waiting for requests. Send traffic to the deployment URL — captured
                      pairs will appear here within a few seconds.
                    </Alert>
                  ) : filteredRequestRows.length === 0 ? (
                    <Alert severity="info">
                      No requests match the active filter. Try widening the status bucket or
                      clearing the path filter.
                    </Alert>
                  ) : (
                    <TableContainer
                      component={Box}
                      sx={{
                        border: 1,
                        borderColor: 'divider',
                        borderRadius: 1,
                        maxHeight: 480,
                      }}
                    >
                      <Table size="small" stickyHeader>
                        <TableHead>
                          <TableRow>
                            <TableCell>Time</TableCell>
                            <TableCell>Method</TableCell>
                            <TableCell>Path</TableCell>
                            <TableCell>Status</TableCell>
                            <TableCell align="right">Latency</TableCell>
                            <TableCell>IP</TableCell>
                          </TableRow>
                        </TableHead>
                        <TableBody>
                          {filteredRequestRows.map((row, idx) => (
                            <TableRow
                              key={`req-${row.request_id || idx}-${row.timestamp}`}
                              hover
                            >
                              <TableCell sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                                {new Date(row.timestamp).toLocaleTimeString()}
                              </TableCell>
                              <TableCell>
                                <Chip
                                  label={row.method}
                                  size="small"
                                  color="primary"
                                  variant="outlined"
                                />
                              </TableCell>
                              <TableCell sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                                {row.path}
                              </TableCell>
                              <TableCell>
                                <Chip
                                  label={row.status}
                                  size="small"
                                  color={
                                    row.status >= 500
                                      ? 'error'
                                      : row.status >= 400
                                        ? 'warning'
                                        : 'success'
                                  }
                                />
                              </TableCell>
                              <TableCell align="right" sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                                {row.latency_ms}ms
                              </TableCell>
                              <TableCell sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                                {row.client_ip ?? '—'}
                              </TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    </TableContainer>
                  )}
                </Box>
              )}

              {/* Captures tab: full request/response pairs from mockforge-recorder via the cloud proxy (#234). */}
              {detailsTab === 4 && (
                <Box>
                  <Box sx={{ display: 'flex', alignItems: 'center', mb: 2, gap: 1, flexWrap: 'wrap' }}>
                    <Typography variant="body2" color="text.secondary" sx={{ flex: 1 }}>
                      Full request/response pairs from the deployment's recorder. Captures live on
                      the deployment's local storage and are wiped on machine restart.
                    </Typography>
                    <Button
                      size="small"
                      variant="outlined"
                      disabled={!selectedDeployment}
                      onClick={async () => {
                        if (!selectedDeployment) return;
                        await toggleRecorder(selectedDeployment.id, 'enable');
                        refetchCaptures();
                      }}
                    >
                      Enable
                    </Button>
                    <Button
                      size="small"
                      variant="outlined"
                      disabled={!selectedDeployment}
                      onClick={async () => {
                        if (!selectedDeployment) return;
                        await toggleRecorder(selectedDeployment.id, 'disable');
                        refetchCaptures();
                      }}
                    >
                      Disable
                    </Button>
                    <Button
                      size="small"
                      variant="outlined"
                      color="warning"
                      disabled={!selectedDeployment || recorderCaptures.length === 0}
                      onClick={async () => {
                        if (!selectedDeployment) return;
                        if (!confirm('Clear all captures on this deployment?')) return;
                        await toggleRecorder(selectedDeployment.id, 'clear');
                        refetchCaptures();
                      }}
                    >
                      Clear
                    </Button>
                    <Button
                      size="small"
                      variant="outlined"
                      disabled={capturesLoading || recorderCaptures.length === 0 || !selectedDeployment}
                      onClick={async () => {
                        if (!selectedDeployment) return;
                        await downloadCapturesHar(selectedDeployment.id, selectedDeployment.slug);
                      }}
                    >
                      Export HAR
                    </Button>
                    <Button
                      size="small"
                      variant="outlined"
                      disabled={capturesLoading || recorderCaptures.length === 0 || !selectedDeployment}
                      onClick={async () => {
                        if (!selectedDeployment) return;
                        await downloadCapturesJsonl(selectedDeployment.id, selectedDeployment.slug);
                      }}
                    >
                      Export JSONL
                    </Button>
                    <Button
                      size="small"
                      variant="outlined"
                      onClick={refetchCaptures}
                      disabled={capturesLoading}
                    >
                      Refresh
                    </Button>
                  </Box>

                  {/* Filter toolbar — mirrors the Requests tab. */}
                  <Box
                    sx={{
                      display: 'flex',
                      alignItems: 'center',
                      flexWrap: 'wrap',
                      gap: 1,
                      mb: 2,
                    }}
                  >
                    {(['all', '2xx', '4xx', '5xx'] as const).map((bucket) => (
                      <Chip
                        key={bucket}
                        label={bucket === 'all' ? 'All' : bucket}
                        size="small"
                        clickable
                        onClick={() => setCapturesStatusFilter(bucket)}
                        color={
                          capturesStatusFilter !== bucket
                            ? 'default'
                            : bucket === '5xx'
                              ? 'error'
                              : bucket === '4xx'
                                ? 'warning'
                                : bucket === '2xx'
                                  ? 'success'
                                  : 'primary'
                        }
                        variant={capturesStatusFilter === bucket ? 'filled' : 'outlined'}
                      />
                    ))}
                    <TextField
                      size="small"
                      placeholder="Filter by path…"
                      value={capturesPathFilter}
                      onChange={(e) => setCapturesPathFilter(e.target.value)}
                      sx={{ minWidth: 240, ml: 'auto' }}
                    />
                  </Box>

                  {capturesError && (
                    <Alert severity="warning" sx={{ mb: 2 }}>
                      {capturesError}. The recorder may not be enabled on this deployment —
                      configure <code>observability.recorder.enabled = true</code> in its config
                      and redeploy.
                    </Alert>
                  )}

                  {(capturesStatusFilter !== 'all' || capturesPathFilter.trim()) &&
                    recorderCaptures.length > 0 && (
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{ display: 'block', mb: 1 }}
                      >
                        Showing {filteredCaptures.length} of {recorderCaptures.length} captures.
                      </Typography>
                    )}

                  {recorderCaptures.length === 0 ? (
                    <Alert severity="info">
                      No captures yet. Enable the recorder on the deployment, send traffic, and
                      the captures will appear here within a few seconds.
                    </Alert>
                  ) : filteredCaptures.length === 0 ? (
                    <Alert severity="info">
                      No captures match the active filter. Try widening the status bucket or
                      clearing the path filter.
                    </Alert>
                  ) : (
                    <TableContainer
                      component={Box}
                      sx={{
                        border: 1,
                        borderColor: 'divider',
                        borderRadius: 1,
                        maxHeight: 480,
                      }}
                    >
                      <Table size="small" stickyHeader>
                        <TableHead>
                          <TableRow>
                            <TableCell>Time</TableCell>
                            <TableCell>Protocol</TableCell>
                            <TableCell>Method</TableCell>
                            <TableCell>Path</TableCell>
                            <TableCell>Status</TableCell>
                            <TableCell align="right">Duration</TableCell>
                            <TableCell />
                          </TableRow>
                        </TableHead>
                        <TableBody>
                          {filteredCaptures.map((capture) => (
                            <TableRow key={capture.id} hover>
                              <TableCell sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                                {new Date(capture.timestamp).toLocaleTimeString()}
                              </TableCell>
                              <TableCell>
                                <Chip
                                  label={String(capture.protocol).toLowerCase()}
                                  size="small"
                                  variant="outlined"
                                />
                              </TableCell>
                              <TableCell>{capture.method}</TableCell>
                              <TableCell sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                                {capture.path}
                              </TableCell>
                              <TableCell>
                                {capture.status_code != null ? (
                                  <Chip
                                    label={capture.status_code}
                                    size="small"
                                    color={
                                      capture.status_code >= 500
                                        ? 'error'
                                        : capture.status_code >= 400
                                          ? 'warning'
                                          : 'success'
                                    }
                                  />
                                ) : (
                                  <Typography variant="caption" color="text.secondary">
                                    —
                                  </Typography>
                                )}
                              </TableCell>
                              <TableCell align="right" sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                                {capture.duration_ms != null ? `${capture.duration_ms}ms` : '—'}
                              </TableCell>
                              <TableCell>
                                <Button size="small" onClick={() => openCapture(capture)}>
                                  View
                                </Button>
                              </TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    </TableContainer>
                  )}
                </Box>
              )}

              {detailsTab === 5 && (
                <Box>
                  <Box sx={{ display: 'flex', alignItems: 'center', mb: 2, gap: 1, flexWrap: 'wrap' }}>
                    <Typography variant="body2" color="text.secondary" sx={{ flex: 1 }}>
                      Distributed traces from this deployment, ingested via OTLP. Spans are kept
                      in the registry's Postgres store and pruned by the retention worker.
                    </Typography>
                    <Button
                      size="small"
                      variant="outlined"
                      onClick={refetchTraces}
                      disabled={tracesLoading}
                    >
                      Refresh
                    </Button>
                  </Box>

                  {tracesError && (
                    <Alert severity="warning" sx={{ mb: 2 }}>
                      {tracesError}. The deployment may not have an OTLP exporter configured —
                      enable <code>observability.tracing.enabled = true</code> with the registry as
                      the OTLP endpoint and redeploy.
                    </Alert>
                  )}

                  {deploymentTraces.length === 0 ? (
                    <Alert severity="info">
                      No traces yet. Once the deployment exports OTLP spans, they'll appear here
                      within a few seconds.
                    </Alert>
                  ) : (
                    <TableContainer
                      component={Box}
                      sx={{
                        border: 1,
                        borderColor: 'divider',
                        borderRadius: 1,
                        maxHeight: 480,
                      }}
                    >
                      <Table size="small" stickyHeader>
                        <TableHead>
                          <TableRow>
                            <TableCell>Time</TableCell>
                            <TableCell>Service</TableCell>
                            <TableCell>Root span</TableCell>
                            <TableCell align="right">Spans</TableCell>
                            <TableCell align="right">Duration</TableCell>
                            <TableCell>Status</TableCell>
                            <TableCell />
                          </TableRow>
                        </TableHead>
                        <TableBody>
                          {deploymentTraces.map((trace) => (
                            <TableRow key={trace.trace_id} hover>
                              <TableCell sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                                {new Date(trace.start).toLocaleTimeString()}
                              </TableCell>
                              <TableCell>{trace.service_name ?? '—'}</TableCell>
                              <TableCell sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                                {trace.root_name}
                              </TableCell>
                              <TableCell align="right">{trace.span_count}</TableCell>
                              <TableCell
                                align="right"
                                sx={{ fontFamily: 'monospace', fontSize: 12 }}
                              >
                                {trace.duration_ms.toFixed(1)}ms
                              </TableCell>
                              <TableCell>
                                {trace.has_error ? (
                                  <Chip label="error" size="small" color="error" />
                                ) : (
                                  <Chip label="ok" size="small" color="success" />
                                )}
                              </TableCell>
                              <TableCell>
                                <Button size="small" onClick={() => openTrace(trace)}>
                                  View
                                </Button>
                              </TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    </TableContainer>
                  )}
                </Box>
              )}

              {detailsTab === 6 && (
                <Box>
                  {streamConnected && streamMetrics && (
                    <Box sx={{ mb: 3 }}>
                      <Alert severity="success" sx={{ mb: 2 }}>
                        Live metrics — updating in real time
                      </Alert>
                      <Grid container spacing={2}>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              Requests / sec
                            </Typography>
                            <Typography variant="h6">{streamMetrics.requests_per_second.toFixed(1)}</Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              Avg Latency
                            </Typography>
                            <Typography variant="h6">{streamMetrics.avg_latency_ms.toFixed(0)} ms</Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              P95 Latency
                            </Typography>
                            <Typography variant="h6">{streamMetrics.p95_latency_ms.toFixed(0)} ms</Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              Error Rate
                            </Typography>
                            <Typography variant="h6" color={streamMetrics.error_rate > 0.05 ? 'error.main' : 'success.main'}>
                              {(streamMetrics.error_rate * 100).toFixed(1)}%
                            </Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              Active Connections
                            </Typography>
                            <Typography variant="h6">{streamMetrics.active_connections}</Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              Total Requests
                            </Typography>
                            <Typography variant="h6">{streamMetrics.total_requests.toLocaleString()}</Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              Total Errors
                            </Typography>
                            <Typography variant="h6" color="error.main">{streamMetrics.total_errors.toLocaleString()}</Typography>
                          </Paper>
                        </Grid>
                      </Grid>
                    </Box>
                  )}
                  {metricsLoading ? (
                    <Box sx={{ display: 'flex', justifyContent: 'center', p: 3 }}>
                      <CircularProgress />
                    </Box>
                  ) : !metrics && !streamMetrics ? (
                    <Alert severity="info">No metrics available</Alert>
                  ) : metrics ? (
                    <Box>
                      <Typography variant="subtitle2" sx={{ mb: 1 }}>
                        Aggregated Metrics
                      </Typography>
                      <Grid container spacing={2}>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              Total Requests
                            </Typography>
                            <Typography variant="h6">{metrics.requests.toLocaleString()}</Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              2xx Responses
                            </Typography>
                            <Typography variant="h6" color="success.main">
                              {metrics.requests_2xx.toLocaleString()}
                            </Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              4xx Responses
                            </Typography>
                            <Typography variant="h6" color="warning.main">
                              {metrics.requests_4xx.toLocaleString()}
                            </Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              5xx Responses
                            </Typography>
                            <Typography variant="h6" color="error.main">
                              {metrics.requests_5xx.toLocaleString()}
                            </Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              Egress (bytes)
                            </Typography>
                            <Typography variant="h6">
                              {(metrics.egress_bytes / 1024 / 1024).toFixed(2)} MB
                            </Typography>
                          </Paper>
                        </Grid>
                        <Grid item xs={6} md={3}>
                          <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                            <Typography variant="caption" color="text.secondary">
                              Avg Response Time
                            </Typography>
                            <Typography variant="h6">{metrics.avg_response_time_ms} ms</Typography>
                          </Paper>
                        </Grid>
                      </Grid>
                    </Box>
                  ) : null}
                </Box>
              )}
            </DialogContent>

            <DialogActions>
              <Button onClick={() => setDetailsOpen(false)}>Close</Button>
              {selectedDeployment.deployment_url && (
                <>
                  <Button
                    variant="contained"
                    startIcon={<CodeIcon />}
                    onClick={() => {
                      setDetailsOpen(false);
                      navigateToExplorer(selectedDeployment);
                    }}
                  >
                    Explore API
                  </Button>
                  <Button
                    startIcon={<OpenInNewIcon />}
                    href={`${selectedDeployment.deployment_url}/__mockforge/docs`}
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    Open in new tab
                  </Button>
                </>
              )}
            </DialogActions>
          </>
        )}
      </Dialog>

      {/* Capture detail dialog. Sibling to the deployment details dialog so
          we don't have to nest dialogs (which MUI handles but produces
          confusing focus behaviour). */}
      <Dialog
        open={!!selectedCapture}
        onClose={() => {
          setSelectedCapture(null);
          setSelectedCaptureResponse(null);
        }}
        maxWidth="lg"
        fullWidth
      >
        {selectedCapture && (
          <>
            <DialogTitle>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Chip label={selectedCapture.method} size="small" variant="outlined" />
                <Typography variant="body1" sx={{ fontFamily: 'monospace' }}>
                  {selectedCapture.path}
                </Typography>
                {selectedCapture.status_code != null && (
                  <Chip
                    label={selectedCapture.status_code}
                    size="small"
                    color={
                      selectedCapture.status_code >= 500
                        ? 'error'
                        : selectedCapture.status_code >= 400
                          ? 'warning'
                          : 'success'
                    }
                  />
                )}
              </Box>
            </DialogTitle>
            <DialogContent dividers>
              <Typography variant="subtitle2" gutterBottom>
                Request
              </Typography>
              <Box
                component="pre"
                sx={{
                  fontFamily: 'monospace',
                  fontSize: 12,
                  bgcolor: 'background.default',
                  border: 1,
                  borderColor: 'divider',
                  borderRadius: 1,
                  p: 1.5,
                  maxHeight: 240,
                  overflow: 'auto',
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                  m: 0,
                }}
              >
                {`Headers:\n${formatJsonString(selectedCapture.headers)}\n\nBody (${selectedCapture.body_encoding}):\n${selectedCapture.body ?? '(empty)'}`}
              </Box>

              <Typography variant="subtitle2" sx={{ mt: 2 }} gutterBottom>
                Response
              </Typography>
              {captureResponseLoading ? (
                <Box sx={{ display: 'flex', justifyContent: 'center', p: 2 }}>
                  <CircularProgress size={20} />
                </Box>
              ) : selectedCaptureResponse ? (
                <Box
                  component="pre"
                  sx={{
                    fontFamily: 'monospace',
                    fontSize: 12,
                    bgcolor: 'background.default',
                    border: 1,
                    borderColor: 'divider',
                    borderRadius: 1,
                    p: 1.5,
                    maxHeight: 240,
                    overflow: 'auto',
                    whiteSpace: 'pre-wrap',
                    wordBreak: 'break-word',
                    m: 0,
                  }}
                >
                  {`Status: ${selectedCaptureResponse.status_code}\n\nHeaders:\n${formatJsonString(selectedCaptureResponse.headers)}\n\nBody (${selectedCaptureResponse.body_encoding}):\n${selectedCaptureResponse.body ?? '(empty)'}`}
                </Box>
              ) : (
                <Alert severity="info">
                  No response body recorded. The deployment may have shut down before the
                  response was committed, or the recorder may not capture response bodies for
                  this protocol.
                </Alert>
              )}

              {(replayResult !== null || replayError || replayLoading) && (
                <Box sx={{ mt: 2 }}>
                  <Typography variant="subtitle2" gutterBottom>
                    Replay result
                  </Typography>
                  {replayLoading ? (
                    <Box sx={{ display: 'flex', justifyContent: 'center', p: 2 }}>
                      <CircularProgress size={20} />
                    </Box>
                  ) : replayError ? (
                    <Alert severity="error">{replayError}</Alert>
                  ) : (
                    <Box
                      component="pre"
                      sx={{
                        fontFamily: 'monospace',
                        fontSize: 12,
                        bgcolor: 'background.default',
                        border: 1,
                        borderColor: 'divider',
                        borderRadius: 1,
                        p: 1.5,
                        maxHeight: 240,
                        overflow: 'auto',
                        whiteSpace: 'pre-wrap',
                        wordBreak: 'break-word',
                        m: 0,
                      }}
                    >
                      {typeof replayResult === 'string'
                        ? replayResult
                        : JSON.stringify(replayResult, null, 2)}
                    </Box>
                  )}
                </Box>
              )}
            </DialogContent>
            <DialogActions>
              <Button
                onClick={replaySelectedCapture}
                disabled={replayLoading || !selectedDeployment}
              >
                Replay
              </Button>
              <Box sx={{ flex: 1 }} />
              <Button
                onClick={() => {
                  setSelectedCapture(null);
                  setSelectedCaptureResponse(null);
                  setReplayResult(null);
                  setReplayError(null);
                }}
              >
                Close
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>

      {/* Trace detail dialog — renders the full span list as an indented
          waterfall. Sibling to the capture and deployment dialogs to keep
          focus management simple. */}
      <Dialog
        open={!!selectedTrace}
        onClose={() => {
          setSelectedTrace(null);
          setSelectedTraceSpans([]);
        }}
        maxWidth="lg"
        fullWidth
      >
        {selectedTrace && (
          <>
            <DialogTitle>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexWrap: 'wrap' }}>
                <Typography variant="body1">{selectedTrace.root_name}</Typography>
                <Typography
                  variant="caption"
                  color="text.secondary"
                  sx={{ fontFamily: 'monospace' }}
                >
                  {selectedTrace.trace_id}
                </Typography>
                {selectedTrace.has_error && <Chip label="error" size="small" color="error" />}
              </Box>
            </DialogTitle>
            <DialogContent>
              {traceSpansLoading ? (
                <Box sx={{ display: 'flex', justifyContent: 'center', p: 3 }}>
                  <CircularProgress />
                </Box>
              ) : selectedTraceSpans.length === 0 ? (
                <Alert severity="info">No spans found for this trace.</Alert>
              ) : (
                <SpanWaterfall spans={selectedTraceSpans} />
              )}
            </DialogContent>
            <DialogActions>
              <Button
                onClick={() => {
                  setSelectedTrace(null);
                  setSelectedTraceSpans([]);
                }}
              >
                Close
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>
    </Box>
  );
};

/**
 * Pretty-print a JSON string. Recorder stores headers and query_params as
 * JSON-encoded strings; if the parse fails we surface the raw text so a
 * malformed entry is still readable.
 */
function formatJsonString(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

/**
 * Renders the spans of a single trace as a waterfall: each row sized and
 * offset relative to the trace's wall-clock window. The visual encoding —
 * colored bar proportional to duration, indented by parent depth — is
 * the standard tracing-UI idiom; we don't try to compete with Jaeger here,
 * just give the user a glanceable shape.
 */
function SpanWaterfall({ spans }: { spans: TraceSpan[] }): React.ReactElement {
  const traceStart = Math.min(...spans.map((s) => s.start_unix_nano));
  const traceEnd = Math.max(...spans.map((s) => s.end_unix_nano));
  const traceDuration = Math.max(1, traceEnd - traceStart);

  // Build parent → children index, then walk it depth-first so the table
  // shows spans in causal order rather than wall-clock order. Falls back
  // to the original list if no parent is identified (e.g. partial trace).
  const childrenOf = new Map<string | null, TraceSpan[]>();
  for (const span of spans) {
    const key = span.parent_span_id ?? null;
    const list = childrenOf.get(key) ?? [];
    list.push(span);
    childrenOf.set(key, list);
  }
  for (const list of childrenOf.values()) {
    list.sort((a, b) => a.start_unix_nano - b.start_unix_nano);
  }

  const ordered: { span: TraceSpan; depth: number }[] = [];
  const visited = new Set<string>();
  const walk = (parentId: string | null, depth: number) => {
    const kids = childrenOf.get(parentId) ?? [];
    for (const span of kids) {
      if (visited.has(span.span_id)) continue;
      visited.add(span.span_id);
      ordered.push({ span, depth });
      walk(span.span_id, depth + 1);
    }
  };
  walk(null, 0);
  // Pick up any orphans whose parent isn't in the result set.
  for (const span of spans) {
    if (!visited.has(span.span_id)) {
      ordered.push({ span, depth: 0 });
      visited.add(span.span_id);
    }
  }

  return (
    <TableContainer component={Box} sx={{ border: 1, borderColor: 'divider', borderRadius: 1 }}>
      <Table size="small">
        <TableHead>
          <TableRow>
            <TableCell>Name</TableCell>
            <TableCell align="right">Start</TableCell>
            <TableCell align="right">Duration</TableCell>
            <TableCell sx={{ width: '40%' }}>Timeline</TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {ordered.map(({ span, depth }) => {
            const startOffset = span.start_unix_nano - traceStart;
            const duration = span.end_unix_nano - span.start_unix_nano;
            const startPct = (startOffset / traceDuration) * 100;
            const widthPct = Math.max(0.5, (duration / traceDuration) * 100);
            const isError = span.status_code === 2;
            return (
              <TableRow key={span.span_id} hover>
                <TableCell sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                  <Box sx={{ pl: depth * 2 }}>
                    {span.name}
                    {isError && (
                      <Chip
                        label="error"
                        size="small"
                        color="error"
                        sx={{ ml: 1, height: 16, fontSize: 10 }}
                      />
                    )}
                  </Box>
                </TableCell>
                <TableCell align="right" sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                  {(startOffset / 1e6).toFixed(2)}ms
                </TableCell>
                <TableCell align="right" sx={{ fontFamily: 'monospace', fontSize: 12 }}>
                  {(duration / 1e6).toFixed(2)}ms
                </TableCell>
                <TableCell>
                  <Box
                    sx={{
                      position: 'relative',
                      height: 14,
                      bgcolor: 'action.hover',
                      borderRadius: 0.5,
                    }}
                  >
                    <Box
                      sx={{
                        position: 'absolute',
                        left: `${startPct}%`,
                        width: `${widthPct}%`,
                        height: '100%',
                        bgcolor: isError ? 'error.main' : 'primary.main',
                        borderRadius: 0.5,
                      }}
                    />
                  </Box>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </TableContainer>
  );
}

/**
 * Trigger a HAR download for a deployment's captures. Goes through the
 * cloud proxy so the auth_token JWT is the only thing the browser needs;
 * the deployment URL stays server-side.
 *
 * Filename embeds slug + ISO date so the user can stash multiple exports
 * without overwriting. Errors are surfaced as alerts because this is a
 * one-shot user action — silent failure would leave them wondering why
 * nothing downloaded.
 */
/**
 * Flip the deployment's recorder enable/disable/clear state via the
 * cloud proxy. We surface server-side errors as alerts because each
 * action is a deliberate user click — silent failure is worse than
 * mildly noisy success.
 */
async function toggleRecorder(
  deploymentId: string,
  action: 'enable' | 'disable' | 'clear',
): Promise<void> {
  const token = localStorage.getItem('auth_token');
  if (!token) {
    alert('Not authenticated');
    return;
  }
  const url = `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/captures/${action}`;
  try {
    const resp = await fetch(url, {
      method: 'POST',
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!resp.ok) {
      throw new Error(`HTTP ${resp.status}`);
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Recorder action failed';
    alert(`Recorder ${action} failed: ${msg}`);
  }
}

async function downloadCapturesHar(deploymentId: string, slug: string): Promise<void> {
  return downloadCaptures(deploymentId, slug, 'har');
}

async function downloadCapturesJsonl(deploymentId: string, slug: string): Promise<void> {
  return downloadCaptures(deploymentId, slug, 'jsonl');
}

async function downloadCaptures(
  deploymentId: string,
  slug: string,
  format: 'har' | 'jsonl',
): Promise<void> {
  const token = localStorage.getItem('auth_token');
  if (!token) {
    alert('Not authenticated');
    return;
  }
  const url = `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/captures/export/${format}`;
  try {
    const resp = await fetch(url, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!resp.ok) {
      throw new Error(`HTTP ${resp.status}`);
    }
    const blob = await resp.blob();
    const objectUrl = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = objectUrl;
    const date = new Date().toISOString().split('T')[0];
    a.download = `${slug}-captures-${date}.${format}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(objectUrl);
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Download failed';
    alert(`${format.toUpperCase()} export failed: ${msg}`);
  }
}
