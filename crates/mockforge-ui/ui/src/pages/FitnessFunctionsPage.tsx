import { logger } from '@/utils/logger';
import React, { useState, useMemo } from 'react';
import {
  Activity,
  Plus,
  Trash2,
  Edit,
  Play,
  CheckCircle2,
  XCircle,
  RefreshCw,
  Settings,
  Globe,
  Folder,
  Tag,
  Route,
  BarChart3,
  TrendingUp,
} from 'lucide-react';
import { driftApi, type FitnessFunction, type FitnessFunctionType, type FitnessScope, type CreateFitnessFunctionRequest, type FitnessTestResult, type DriftIncident } from '../services/driftApi';
import { useDriftIncidents } from '../hooks/useApi';
import {
  cloudContractApi,
  type FitnessFunction as CloudFitnessFunction,
  type CreateFitnessFunctionRequest as CloudCreateFitnessFunctionRequest,
} from '../services/api/cloudContract';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import {
  PageHeader,
  ModernCard,
  ModernBadge,
  Alert,
  EmptyState,
  Section,
} from '../components/ui/DesignSystem';
import { Input } from '../components/ui/input';
import { Button } from '../components/ui/button';
import { Label } from '../components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../components/ui/select';
import { Textarea } from '../components/ui/textarea';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Switch } from '../components/ui/switch';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '../components/ui/Dialog';

// Scope badge component
function ScopeBadge({ scope }: { scope: FitnessFunction['scope'] }) {
  const icons = {
    global: Globe,
    workspace: Folder,
    service: Tag,
    endpoint: Route,
  };

  const labels = {
    global: 'Global',
    workspace: 'Workspace',
    service: 'Service',
    endpoint: 'Endpoint',
  };

  const Icon = icons[scope.type] || Globe;
  const label = scope.type === 'workspace'
    ? `Workspace: ${scope.workspace_id}`
    : scope.type === 'service'
    ? `Service: ${scope.service_name}`
    : scope.type === 'endpoint'
    ? `Endpoint: ${scope.pattern}`
    : labels[scope.type];

  return (
    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-info-100 text-info-700 dark:bg-info-900/20 dark:text-info-300">
      <Icon className="w-3 h-3" />
      {label}
    </span>
  );
}

// Function type badge
function FunctionTypeBadge({ type }: { type: FitnessFunction['function_type'] }) {
  const labels: Record<string, string> = {
    response_size: 'Response Size',
    required_field: 'Required Field',
    field_count: 'Field Count',
    schema_complexity: 'Schema Complexity',
    custom: 'Custom',
  };

  return (
    <span className="px-2.5 py-1 rounded-full text-xs font-medium bg-purple-100 text-purple-800 dark:bg-purple-900/20 dark:text-purple-300">
      {labels[type.type] || type.type}
    </span>
  );
}

// Fitness function row component
function FitnessFunctionRow({
  function: func,
  onEdit,
  onDelete,
  onTest,
  readOnly = false,
  hideTest = false,
}: {
  function: FitnessFunction;
  onEdit: (func: FitnessFunction) => void;
  onDelete: (id: string) => void;
  onTest: (id: string) => void;
  /** Hide the entire action group. Reserved for genuinely read-only views. */
  readOnly?: boolean;
  /** Hide only the Test button — used in cloud mode where the test-now
   *  endpoint isn't wired yet, while edit/delete still work. */
  hideTest?: boolean;
}) {
  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  return (
    <div className="border-b border-border last:border-b-0 hover:bg-accent hover:text-accent-foreground/50 transition-colors">
      <div className="p-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-3 mb-2">
              <h3 className="text-sm font-semibold text-foreground">
                {func.name}
              </h3>
              <FunctionTypeBadge type={func.function_type} />
              <ScopeBadge scope={func.scope} />
              {func.enabled ? (
                <ModernBadge variant="success" size="sm">Enabled</ModernBadge>
              ) : (
                <ModernBadge variant="outline" size="sm">Disabled</ModernBadge>
              )}
            </div>

            <p className="text-sm text-muted-foreground mb-2">
              {func.description}
            </p>

            <div className="flex items-center gap-4 text-xs text-muted-foreground">
              <span>Created: {formatDate(func.created_at)}</span>
              <span>Updated: {formatDate(func.updated_at)}</span>
            </div>
          </div>

          {!readOnly && (
            <div className="flex items-center gap-2">
              {!hideTest && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => onTest(func.id)}
                >
                  <Play className="w-4 h-4 mr-1" />
                  Test
                </Button>
              )}
              <Button
                variant="outline"
                size="sm"
                onClick={() => onEdit(func)}
              >
                <Edit className="w-4 h-4 mr-1" />
                Edit
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => onDelete(func.id)}
                className="text-danger-600 hover:text-danger-700 hover:bg-danger-50"
              >
                <Trash2 className="w-4 h-4" />
              </Button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// Fitness function form component
function FitnessFunctionForm({
  function: func,
  onSave,
  onCancel,
}: {
  function: FitnessFunction | null;
  onSave: (request: CreateFitnessFunctionRequest) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(func?.name || '');
  const [description, setDescription] = useState(func?.description || '');
  const [functionType, setFunctionType] = useState<FitnessFunction['function_type']['type']>(
    func?.function_type.type || 'response_size'
  );
  const [enabled, setEnabled] = useState(func?.enabled ?? true);

  // Type-specific config
  const [maxIncreasePercent, setMaxIncreasePercent] = useState(
    func?.function_type.type === 'response_size' ? func.function_type.max_increase_percent.toString() : '25'
  );
  const [pathPattern, setPathPattern] = useState(
    func?.function_type.type === 'required_field' ? func.function_type.path_pattern : ''
  );
  const [allowNewRequired, setAllowNewRequired] = useState(
    func?.function_type.type === 'required_field' ? func.function_type.allow_new_required : false
  );
  const [maxFields, setMaxFields] = useState(
    func?.function_type.type === 'field_count' ? func.function_type.max_fields.toString() : '100'
  );
  const [maxDepth, setMaxDepth] = useState(
    func?.function_type.type === 'schema_complexity' ? func.function_type.max_depth.toString() : '10'
  );

  // Scope
  const [scopeType, setScopeType] = useState<FitnessFunction['scope']['type']>(
    func?.scope.type || 'global'
  );
  const [workspaceId, setWorkspaceId] = useState(
    func?.scope.type === 'workspace' ? func.scope.workspace_id : ''
  );
  const [serviceName, setServiceName] = useState(
    func?.scope.type === 'service' ? func.scope.service_name : ''
  );
  const [endpointPattern, setEndpointPattern] = useState(
    func?.scope.type === 'endpoint' ? func.scope.pattern : ''
  );

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    let functionTypeData: FitnessFunction['function_type'];
    switch (functionType) {
      case 'response_size':
        functionTypeData = {
          type: 'response_size',
          max_increase_percent: parseFloat(maxIncreasePercent) || 25,
        };
        break;
      case 'required_field':
        functionTypeData = {
          type: 'required_field',
          path_pattern: pathPattern,
          allow_new_required: allowNewRequired,
        };
        break;
      case 'field_count':
        functionTypeData = {
          type: 'field_count',
          max_fields: parseInt(maxFields) || 100,
        };
        break;
      case 'schema_complexity':
        functionTypeData = {
          type: 'schema_complexity',
          max_depth: parseInt(maxDepth) || 10,
        };
        break;
      default:
        functionTypeData = {
          type: 'response_size',
          max_increase_percent: 25,
        };
    }

    let scopeData: FitnessFunction['scope'];
    switch (scopeType) {
      case 'workspace':
        scopeData = { type: 'workspace', workspace_id: workspaceId };
        break;
      case 'service':
        scopeData = { type: 'service', service_name: serviceName };
        break;
      case 'endpoint':
        scopeData = { type: 'endpoint', pattern: endpointPattern };
        break;
      default:
        scopeData = { type: 'global' };
    }

    onSave({
      name,
      description,
      function_type: functionTypeData,
      config: {},
      scope: scopeData,
      enabled,
    });
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div>
        <Label>Name</Label>
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="e.g., Mobile API Response Size Limit"
          required
        />
      </div>

      <div>
        <Label>Description</Label>
        <Textarea
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Describe what this fitness function checks..."
          rows={3}
          required
        />
      </div>

      <div>
        <Label>Function Type</Label>
        <Select value={functionType} onValueChange={(v) => setFunctionType(v as any)}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="response_size">Response Size</SelectItem>
            <SelectItem value="required_field">Required Field</SelectItem>
            <SelectItem value="field_count">Field Count</SelectItem>
            <SelectItem value="schema_complexity">Schema Complexity</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Type-specific configuration */}
      {functionType === 'response_size' && (
        <div>
          <Label>Max Increase Percent</Label>
          <Input
            type="number"
            value={maxIncreasePercent}
            onChange={(e) => setMaxIncreasePercent(e.target.value)}
            placeholder="25"
            required
          />
          <p className="text-xs text-muted-foreground mt-1">
            Maximum allowed response size increase percentage (e.g., 25 for 25%)
          </p>
        </div>
      )}

      {functionType === 'required_field' && (
        <>
          <div>
            <Label>Path Pattern</Label>
            <Input
              value={pathPattern}
              onChange={(e) => setPathPattern(e.target.value)}
              placeholder="/v1/mobile/*"
              required
            />
            <p className="text-xs text-muted-foreground mt-1">
              Endpoint pattern to check (supports * wildcard)
            </p>
          </div>
          <div className="flex items-center space-x-2">
            <Switch
              id="allow-new-required"
              checked={allowNewRequired}
              onCheckedChange={setAllowNewRequired}
            />
            <Label htmlFor="allow-new-required">Allow new required fields</Label>
          </div>
        </>
      )}

      {functionType === 'field_count' && (
        <div>
          <Label>Max Fields</Label>
          <Input
            type="number"
            value={maxFields}
            onChange={(e) => setMaxFields(e.target.value)}
            placeholder="100"
            required
          />
          <p className="text-xs text-muted-foreground mt-1">
            Maximum number of fields allowed
          </p>
        </div>
      )}

      {functionType === 'schema_complexity' && (
        <div>
          <Label>Max Depth</Label>
          <Input
            type="number"
            value={maxDepth}
            onChange={(e) => setMaxDepth(e.target.value)}
            placeholder="10"
            required
          />
          <p className="text-xs text-muted-foreground mt-1">
            Maximum schema depth allowed
          </p>
        </div>
      )}

      <div>
        <Label>Scope</Label>
        <Select value={scopeType} onValueChange={(v) => setScopeType(v as any)}>
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="global">Global</SelectItem>
            <SelectItem value="workspace">Workspace</SelectItem>
            <SelectItem value="service">Service</SelectItem>
            <SelectItem value="endpoint">Endpoint</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {scopeType === 'workspace' && (
        <div>
          <Label>Workspace ID</Label>
          <Input
            value={workspaceId}
            onChange={(e) => setWorkspaceId(e.target.value)}
            placeholder="workspace-1"
            required
          />
        </div>
      )}

      {scopeType === 'service' && (
        <div>
          <Label>Service Name</Label>
          <Input
            value={serviceName}
            onChange={(e) => setServiceName(e.target.value)}
            placeholder="user-service"
            required
          />
        </div>
      )}

      {scopeType === 'endpoint' && (
        <div>
          <Label>Endpoint Pattern</Label>
          <Input
            value={endpointPattern}
            onChange={(e) => setEndpointPattern(e.target.value)}
            placeholder="/v1/mobile/*"
            required
          />
        </div>
      )}

      <div className="flex items-center space-x-2">
        <Switch
          id="enabled"
          checked={enabled}
          onCheckedChange={setEnabled}
        />
        <Label htmlFor="enabled">Enabled</Label>
      </div>

      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit">
          {func ? 'Update' : 'Create'} Fitness Function
        </Button>
      </div>
    </form>
  );
}

// Global fitness summary component
function GlobalFitnessSummary({ incidents }: { incidents: DriftIncident[] }) {
  // Aggregate fitness test results from all incidents
  const summary = useMemo(() => {
    let totalTests = 0;
    let passedTests = 0;
    let failedTests = 0;
    const endpointResults: Map<string, { passed: number; failed: number; total: number }> = new Map();
    const functionResults: Map<string, { passed: number; failed: number; total: number }> = new Map();

    incidents.forEach((incident) => {
      if (Array.isArray(incident.fitness_test_results) && incident.fitness_test_results.length > 0) {
        const endpointKey = `${incident.method} ${incident.endpoint}`;

        incident.fitness_test_results.forEach((result) => {
          totalTests++;
          if (result.passed) {
            passedTests++;
          } else {
            failedTests++;
          }

          // Aggregate by endpoint
          const endpointStats = endpointResults.get(endpointKey) || { passed: 0, failed: 0, total: 0 };
          endpointStats.total++;
          if (result.passed) {
            endpointStats.passed++;
          } else {
            endpointStats.failed++;
          }
          endpointResults.set(endpointKey, endpointStats);

          // Aggregate by function
          const functionName = result.function_name || result.function_id;
          const functionStats = functionResults.get(functionName) || { passed: 0, failed: 0, total: 0 };
          functionStats.total++;
          if (result.passed) {
            functionStats.passed++;
          } else {
            functionStats.failed++;
          }
          functionResults.set(functionName, functionStats);
        });
      }
    });

    return {
      totalTests,
      passedTests,
      failedTests,
      passRate: totalTests > 0 ? (passedTests / totalTests) * 100 : 0,
      endpointResults: Array.from(endpointResults.entries()).map(([endpoint, stats]) => ({
        endpoint,
        ...stats,
      })),
      functionResults: Array.from(functionResults.entries()).map(([functionName, stats]) => ({
        functionName,
        ...stats,
      })),
    };
  }, [incidents]);

  if (summary.totalTests === 0) {
    return (
      <EmptyState
        icon={<Activity className="w-6 h-6" />}
        title="No Fitness Test Results"
        description="Fitness test results will appear here once contract drift is detected"
      />
    );
  }

  return (
    <div className="space-y-6">
      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <ModernCard className="p-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground">Total Tests</p>
              <p className="text-2xl font-bold text-foreground mt-1">
                {summary.totalTests}
              </p>
            </div>
            <div className="p-3 bg-info-100 dark:bg-info-900/20 rounded-lg">
              <BarChart3 className="w-6 h-6 text-info-600 dark:text-info-400" />
            </div>
          </div>
        </ModernCard>

        <ModernCard className="p-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground">Passed</p>
              <p className="text-2xl font-bold text-success-600 dark:text-success-400 mt-1">
                {summary.passedTests}
              </p>
            </div>
            <div className="p-3 bg-success-100 dark:bg-success-900/20 rounded-lg">
              <CheckCircle2 className="w-6 h-6 text-success-600 dark:text-success-400" />
            </div>
          </div>
        </ModernCard>

        <ModernCard className="p-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground">Failed</p>
              <p className="text-2xl font-bold text-danger-600 dark:text-danger-400 mt-1">
                {summary.failedTests}
              </p>
            </div>
            <div className="p-3 bg-danger-100 dark:bg-danger-900/20 rounded-lg">
              <XCircle className="w-6 h-6 text-danger-600 dark:text-danger-400" />
            </div>
          </div>
        </ModernCard>

        <ModernCard className="p-4">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground">Pass Rate</p>
              <p className="text-2xl font-bold text-foreground mt-1">
                {summary.passRate.toFixed(1)}%
              </p>
            </div>
            <div className="p-3 bg-purple-100 dark:bg-purple-900/20 rounded-lg">
              <TrendingUp className="w-6 h-6 text-purple-600 dark:text-purple-400" />
            </div>
          </div>
        </ModernCard>
      </div>

      {/* Per-Endpoint Results */}
      {summary.endpointResults.length > 0 && (
        <Section title="Per-Endpoint Fitness Results" subtitle="Fitness test results grouped by endpoint">
          <ModernCard>
            <div className="overflow-x-auto">
              <table className="w-full border-collapse">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left p-3 font-semibold text-sm text-foreground">Endpoint</th>
                    <th className="text-left p-3 font-semibold text-sm text-foreground">Total</th>
                    <th className="text-left p-3 font-semibold text-sm text-foreground">Passed</th>
                    <th className="text-left p-3 font-semibold text-sm text-foreground">Failed</th>
                    <th className="text-left p-3 font-semibold text-sm text-foreground">Pass Rate</th>
                  </tr>
                </thead>
                <tbody>
                  {summary.endpointResults.map((result, idx) => {
                    const passRate = result.total > 0 ? (result.passed / result.total) * 100 : 0;
                    return (
                      <tr key={idx} className="border-b border-border hover:bg-accent hover:text-accent-foreground/50">
                        <td className="p-3 text-sm font-mono text-foreground">{result.endpoint}</td>
                        <td className="p-3 text-sm text-muted-foreground">{result.total}</td>
                        <td className="p-3 text-sm text-success-600 dark:text-success-400">{result.passed}</td>
                        <td className="p-3 text-sm text-danger-600 dark:text-danger-400">{result.failed}</td>
                        <td className="p-3">
                          <span className={`text-sm font-semibold ${passRate >= 80 ? 'text-success-600 dark:text-success-400' : passRate >= 50 ? 'text-warning-600 dark:text-warning-400' : 'text-danger-600 dark:text-danger-400'}`}>
                            {passRate.toFixed(1)}%
                          </span>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </ModernCard>
        </Section>
      )}

      {/* Per-Function Results */}
      {summary.functionResults.length > 0 && (
        <Section title="Per-Function Results" subtitle="Fitness test results grouped by function">
          <ModernCard>
            <div className="overflow-x-auto">
              <table className="w-full border-collapse">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left p-3 font-semibold text-sm text-foreground">Function</th>
                    <th className="text-left p-3 font-semibold text-sm text-foreground">Total</th>
                    <th className="text-left p-3 font-semibold text-sm text-foreground">Passed</th>
                    <th className="text-left p-3 font-semibold text-sm text-foreground">Failed</th>
                    <th className="text-left p-3 font-semibold text-sm text-foreground">Pass Rate</th>
                  </tr>
                </thead>
                <tbody>
                  {summary.functionResults.map((result, idx) => {
                    const passRate = result.total > 0 ? (result.passed / result.total) * 100 : 0;
                    return (
                      <tr key={idx} className="border-b border-border hover:bg-accent hover:text-accent-foreground/50">
                        <td className="p-3 text-sm text-foreground">{result.functionName}</td>
                        <td className="p-3 text-sm text-muted-foreground">{result.total}</td>
                        <td className="p-3 text-sm text-success-600 dark:text-success-400">{result.passed}</td>
                        <td className="p-3 text-sm text-danger-600 dark:text-danger-400">{result.failed}</td>
                        <td className="p-3">
                          <span className={`text-sm font-semibold ${passRate >= 80 ? 'text-success-600 dark:text-success-400' : passRate >= 50 ? 'text-warning-600 dark:text-warning-400' : 'text-danger-600 dark:text-danger-400'}`}>
                            {passRate.toFixed(1)}%
                          </span>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </ModernCard>
        </Section>
      )}
    </div>
  );
}

// Fold the local typed shape (`function_type` + `scope`) back into the
// cloud's flat `{name, kind, config}` payload. Goes the opposite
// direction from `adaptCloudFitnessFunction`. Symmetry matters for
// edit: a cloud row read → adapted to local → user edits → folded back
// for the PATCH must round-trip without dropping config keys, so
// `function_type` and `scope` ride inside `config` alongside whatever
// kind-specific fields the form added.
function localToCloudFitnessFunction(
  request: CreateFitnessFunctionRequest,
): CloudCreateFitnessFunctionRequest {
  return {
    name: request.name,
    kind: request.function_type.type,
    config: {
      ...(request.config ?? {}),
      // Persist the rich typing alongside the raw config so a future
      // round-trip back to the local shape can reconstruct exactly
      // what the user picked, instead of having `adaptCloudFitnessFunction`
      // guess from `kind`.
      function_type: request.function_type,
      scope: request.scope,
      // `description` and `enabled` aren't part of the backend schema
      // (no columns), but the local form collects them. Stashing inside
      // config keeps them visible to a future edit without losing data;
      // the cloud-side `FitnessFunction` interface already exposes
      // `description` + `enabled` as optional fields, so the read-side
      // adapter can pull these back out when they were set.
      description: request.description,
      enabled: request.enabled ?? true,
    },
  };
}

// Adapt the generic cloud FitnessFunction (kind + config blob) into the
// richer typed shape the local UI expects. Mapping is best-effort —
// extra fields the local UI added when writing (function_type, scope,
// description, enabled) are pulled back out of `config` if present.
function adaptCloudFitnessFunction(cf: CloudFitnessFunction): FitnessFunction {
  const cfg = (cf.config ?? {}) as Record<string, unknown>;
  const fnType = cfg.function_type as FitnessFunctionType | undefined;
  const scope = cfg.scope as FitnessScope | undefined;
  const toEpoch = (iso: string): number => {
    const t = Date.parse(iso);
    return Number.isFinite(t) ? Math.floor(t / 1000) : 0;
  };
  return {
    id: cf.id,
    name: cf.name,
    description: cf.description ?? '',
    function_type:
      fnType ??
      ({ type: cf.kind as FitnessFunctionType['type'] } as FitnessFunctionType),
    config: cfg,
    scope: scope ?? { type: 'workspace', workspace_id: cf.workspace_id },
    enabled: cf.enabled,
    created_at: toEpoch(cf.created_at),
    updated_at: toEpoch(cf.updated_at),
  };
}

export function FitnessFunctionsPage() {
  const queryClient = useQueryClient();
  const [editingFunction, setEditingFunction] = useState<FitnessFunction | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [testResults, setTestResults] = useState<FitnessTestResult[] | null>(null);
  const [showTestResults, setShowTestResults] = useState(false);
  const [showSummary, setShowSummary] = useState(true);
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);
  const cloudMode = isCloudMode();

  // Fetch fitness functions. In cloud mode we hit cloudContractApi
  // (read-only) and adapt rows into the local shape; mutations are
  // disabled until the registry exposes write endpoints.
  const { data, isLoading, refetch } = useQuery({
    queryKey: cloudMode
      ? ['fitness-functions', 'cloud', activeWorkspace?.id ?? '']
      : ['fitness-functions'],
    queryFn: async () => {
      if (cloudMode) {
        if (!activeWorkspace?.id) {
          return { functions: [] as FitnessFunction[] };
        }
        const cloudFns = await cloudContractApi.listFitnessFunctions(
          activeWorkspace.id,
        );
        return { functions: cloudFns.map(adaptCloudFitnessFunction) };
      }
      return driftApi.listFitnessFunctions();
    },
  });

  // Fetch incidents to aggregate fitness results
  const { data: incidentsData } = useDriftIncidents({}, { refetchInterval: 10000 });
  const incidents = incidentsData?.incidents || [];

  // Invalidation key — must match the queryKey used above so create/
  // update/delete refresh the right cache slot regardless of mode.
  const fitnessFunctionsQueryKey: (string | undefined)[] = cloudMode
    ? ['fitness-functions', 'cloud', activeWorkspace?.id ?? '']
    : ['fitness-functions'];

  // Create mutation. In cloud mode, the workspace id is required by the
  // backend route; without it we surface a clear error before queueing
  // a request that would fail on the server.
  const createMutation = useMutation({
    mutationFn: (request: CreateFitnessFunctionRequest) => {
      if (cloudMode) {
        if (!activeWorkspace?.id) {
          return Promise.reject(
            new Error('Pick a workspace before creating a fitness function.'),
          );
        }
        return cloudContractApi.createFitnessFunction(
          activeWorkspace.id,
          localToCloudFitnessFunction(request),
        );
      }
      return driftApi.createFitnessFunction(request);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: fitnessFunctionsQueryKey });
      setShowForm(false);
      setEditingFunction(null);
    },
    onError: (error: Error) => {
      logger.error('Failed to create fitness function', error);
      alert(`Failed to create fitness function: ${error.message}`);
    },
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: ({ id, request }: { id: string; request: CreateFitnessFunctionRequest }) => {
      if (cloudMode) {
        return cloudContractApi.updateFitnessFunction(
          id,
          localToCloudFitnessFunction(request),
        );
      }
      return driftApi.updateFitnessFunction(id, request);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: fitnessFunctionsQueryKey });
      setShowForm(false);
      setEditingFunction(null);
    },
    onError: (error: Error) => {
      logger.error('Failed to update fitness function', error);
      alert(`Failed to update fitness function: ${error.message}`);
    },
  });

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: (id: string) => {
      if (cloudMode) {
        return cloudContractApi.deleteFitnessFunction(id);
      }
      return driftApi.deleteFitnessFunction(id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: fitnessFunctionsQueryKey });
    },
    onError: (error: Error) => {
      logger.error('Failed to delete fitness function', error);
      alert(`Failed to delete fitness function: ${error.message}`);
    },
  });

  // Test mutation
  const testMutation = useMutation({
    mutationFn: (id: string) => driftApi.testFitnessFunction(id, {}),
    onSuccess: (data) => {
      setTestResults(data.results || []);
      setShowTestResults(true);
    },
    onError: (error: Error) => {
      logger.error('Failed to test fitness function', error);
      alert(`Failed to test fitness function: ${error.message}`);
    },
  });

  const handleSave = (request: CreateFitnessFunctionRequest) => {
    if (editingFunction) {
      updateMutation.mutate({ id: editingFunction.id, request });
    } else {
      createMutation.mutate(request);
    }
  };

  const handleEdit = (func: FitnessFunction) => {
    setEditingFunction(func);
    setShowForm(true);
  };

  const handleDelete = (id: string) => {
    setPendingDeleteId(id);
  };

  const confirmDelete = () => {
    if (pendingDeleteId) {
      deleteMutation.mutate(pendingDeleteId, {
        onSettled: () => setPendingDeleteId(null),
      });
    }
  };

  const handleTest = (id: string) => {
    testMutation.mutate(id);
  };

  const functions = data?.functions || [];

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title="Fitness Functions"
        description="Register custom tests that run against each new contract version to enforce constraints"
        icon={<Activity className="w-6 h-6" />}
      />

      {cloudMode && (
        <Alert variant="info">
          Cloud fitness functions support create, edit, and delete via the
          registry. The <strong>Test</strong> button is currently
          local-only — until the cloud-side evaluator lands, evaluation
          happens on a schedule via the test-runner.
        </Alert>
      )}

      <div className="flex justify-between items-center">
        <div className="text-sm text-muted-foreground">
          {functions.length} fitness function{functions.length !== 1 ? 's' : ''} registered
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={() => setShowSummary(!showSummary)}>
            {showSummary ? 'Hide' : 'Show'} Summary
          </Button>
          <Button
            onClick={() => {
              setEditingFunction(null);
              setShowForm(true);
            }}
            disabled={cloudMode && !activeWorkspace?.id}
            title={
              cloudMode && !activeWorkspace?.id
                ? 'Pick a workspace before creating a fitness function.'
                : undefined
            }
          >
            <Plus className="w-4 h-4 mr-2" />
            Create Fitness Function
          </Button>
        </div>
      </div>

      {/* Global Fitness Summary */}
      {showSummary && (
        <Section title="Global Fitness Summary" subtitle="Aggregate fitness test results across all endpoints">
          <GlobalFitnessSummary incidents={incidents} />
        </Section>
      )}

      {/* Fitness Functions List */}
      <Section title="Registered Fitness Functions">
        {isLoading ? (
          <div className="p-4 text-center text-muted-foreground">Loading...</div>
        ) : functions.length === 0 ? (
          <EmptyState
            icon={<Activity className="w-6 h-6" />}
            title="No Fitness Functions"
            description="Create your first fitness function to start enforcing contract constraints"
          />
        ) : (
          <div className="border border-border rounded-lg divide-y divide-border">
            {(Array.isArray(functions) ? functions : []).map((func) => (
              <FitnessFunctionRow
                key={func.id}
                function={func}
                onEdit={handleEdit}
                onDelete={handleDelete}
                onTest={handleTest}
                hideTest={cloudMode}
              />
            ))}
          </div>
        )}

        <div className="mt-4">
          <Button variant="outline" onClick={() => refetch()}>
            <RefreshCw className="w-4 h-4 mr-2" />
            Refresh
          </Button>
        </div>
      </Section>

      {/* Create/Edit Form Dialog */}
      <Dialog open={showForm} onOpenChange={setShowForm}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>
              {editingFunction ? 'Edit Fitness Function' : 'Create Fitness Function'}
            </DialogTitle>
            <DialogDescription>
              {editingFunction
                ? 'Update the fitness function configuration'
                : 'Register a new fitness function to test contract changes'}
            </DialogDescription>
          </DialogHeader>
          <FitnessFunctionForm
            function={editingFunction}
            onSave={handleSave}
            onCancel={() => {
              setShowForm(false);
              setEditingFunction(null);
            }}
          />
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={pendingDeleteId !== null} onOpenChange={(open) => { if (!open) setPendingDeleteId(null); }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Fitness Function</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this fitness function? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setPendingDeleteId(null)}>
              Cancel
            </Button>
            <Button
              onClick={confirmDelete}
              disabled={deleteMutation.isPending}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {deleteMutation.isPending ? 'Deleting…' : 'Delete'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Test Results Dialog */}
      <Dialog open={showTestResults} onOpenChange={setShowTestResults}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Test Results</DialogTitle>
            <DialogDescription>
              Results from testing the fitness function
            </DialogDescription>
          </DialogHeader>
          {testResults && testResults.length > 0 ? (
            <div className="space-y-4">
              {testResults.map((result, idx) => (
                <ModernCard key={idx}>
                  <div className="flex items-start justify-between mb-2">
                    <div className="flex-1">
                      <h4 className="font-semibold text-foreground">
                        {result.function_name}
                      </h4>
                      <p className="text-sm text-muted-foreground mt-1">
                        {result.message}
                      </p>
                    </div>
                    {result.passed ? (
                      <CheckCircle2 className="w-5 h-5 text-success-500" />
                    ) : (
                      <XCircle className="w-5 h-5 text-danger-500" />
                    )}
                  </div>
                  {result.metrics && typeof result.metrics === 'object' && Object.keys(result.metrics).length > 0 && (
                    <div className="mt-3 pt-3 border-t border-border">
                      <p className="text-xs font-semibold text-muted-foreground mb-2">
                        Metrics:
                      </p>
                      <div className="grid grid-cols-2 gap-2">
                        {Object.entries(result.metrics).map(([key, value]) => (
                          <div key={key} className="text-xs">
                            <span className="text-muted-foreground">{key}:</span>{' '}
                            <span className="font-mono font-semibold">{typeof value === 'number' ? value.toFixed(2) : String(value ?? '')}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </ModernCard>
              ))}
            </div>
          ) : (
            <EmptyState
              icon={<Activity className="w-6 h-6" />}
              title="No Test Results"
              description="No results available"
            />
          )}
          <DialogFooter>
            <Button onClick={() => setShowTestResults(false)}>Close</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
