import { logger } from '@/utils/logger';
import React, { useState } from 'react';
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
} from 'lucide-react';
import { driftApi, type FitnessFunction, type CreateFitnessFunctionRequest, type FitnessTestResult } from '../services/driftApi';
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
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '../components/ui/dialog';

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
    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-300">
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
}: {
  function: FitnessFunction;
  onEdit: (func: FitnessFunction) => void;
  onDelete: (id: string) => void;
  onTest: (id: string) => void;
}) {
  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  return (
    <div className="border-b border-gray-200 dark:border-gray-700 last:border-b-0 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
      <div className="p-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-3 mb-2">
              <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
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

            <p className="text-sm text-gray-600 dark:text-gray-400 mb-2">
              {func.description}
            </p>

            <div className="flex items-center gap-4 text-xs text-gray-500 dark:text-gray-400">
              <span>Created: {formatDate(func.created_at)}</span>
              <span>Updated: {formatDate(func.updated_at)}</span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => onTest(func.id)}
            >
              <Play className="w-4 h-4 mr-1" />
              Test
            </Button>
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
              className="text-red-600 hover:text-red-700 hover:bg-red-50"
            >
              <Trash2 className="w-4 h-4" />
            </Button>
          </div>
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
          <p className="text-xs text-gray-500 mt-1">
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
            <p className="text-xs text-gray-500 mt-1">
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
          <p className="text-xs text-gray-500 mt-1">
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
          <p className="text-xs text-gray-500 mt-1">
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

export function FitnessFunctionsPage() {
  const queryClient = useQueryClient();
  const [editingFunction, setEditingFunction] = useState<FitnessFunction | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [testResults, setTestResults] = useState<FitnessTestResult[] | null>(null);
  const [showTestResults, setShowTestResults] = useState(false);

  // Fetch fitness functions
  const { data, isLoading, refetch } = useQuery({
    queryKey: ['fitness-functions'],
    queryFn: () => driftApi.listFitnessFunctions(),
  });

  // Create mutation
  const createMutation = useMutation({
    mutationFn: (request: CreateFitnessFunctionRequest) => driftApi.createFitnessFunction(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['fitness-functions'] });
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
    mutationFn: ({ id, request }: { id: string; request: CreateFitnessFunctionRequest }) =>
      driftApi.updateFitnessFunction(id, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['fitness-functions'] });
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
    mutationFn: (id: string) => driftApi.deleteFitnessFunction(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['fitness-functions'] });
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
    if (confirm('Are you sure you want to delete this fitness function?')) {
      deleteMutation.mutate(id);
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
        icon={Activity}
      />

      <div className="flex justify-between items-center">
        <div className="text-sm text-gray-600 dark:text-gray-400">
          {functions.length} fitness function{functions.length !== 1 ? 's' : ''} registered
        </div>
        <Button onClick={() => {
          setEditingFunction(null);
          setShowForm(true);
        }}>
          <Plus className="w-4 h-4 mr-2" />
          Create Fitness Function
        </Button>
      </div>

      {/* Fitness Functions List */}
      <Section title="Registered Fitness Functions">
        {isLoading ? (
          <div className="p-4 text-center text-gray-500">Loading...</div>
        ) : functions.length === 0 ? (
          <EmptyState
            icon={Activity}
            title="No Fitness Functions"
            description="Create your first fitness function to start enforcing contract constraints"
          />
        ) : (
          <div className="border border-gray-200 dark:border-gray-700 rounded-lg divide-y divide-gray-200 dark:divide-gray-700">
            {functions.map((func) => (
              <FitnessFunctionRow
                key={func.id}
                function={func}
                onEdit={handleEdit}
                onDelete={handleDelete}
                onTest={handleTest}
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
                      <h4 className="font-semibold text-gray-900 dark:text-gray-100">
                        {result.function_name}
                      </h4>
                      <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                        {result.message}
                      </p>
                    </div>
                    {result.passed ? (
                      <CheckCircle2 className="w-5 h-5 text-green-500" />
                    ) : (
                      <XCircle className="w-5 h-5 text-red-500" />
                    )}
                  </div>
                  {Object.keys(result.metrics).length > 0 && (
                    <div className="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
                      <p className="text-xs font-semibold text-gray-500 dark:text-gray-400 mb-2">
                        Metrics:
                      </p>
                      <div className="grid grid-cols-2 gap-2">
                        {Object.entries(result.metrics).map(([key, value]) => (
                          <div key={key} className="text-xs">
                            <span className="text-gray-500 dark:text-gray-400">{key}:</span>{' '}
                            <span className="font-mono font-semibold">{value.toFixed(2)}</span>
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
              icon={Activity}
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
