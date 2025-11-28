/**
 * Proxy Inspector Component
 *
 * Provides a UI for viewing intercepted requests/responses and managing
 * proxy replacement rules for browser proxy mode.
 */

import React, { useState, useMemo } from 'react';
import { Card } from '../ui/Card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Badge } from '../ui/DesignSystem';
import { ResponsiveTable, type ResponsiveTableColumn } from '../ui/ResponsiveTable';
import { SkeletonTable } from '../ui/Skeleton';
import { DataErrorFallback } from '../error/ErrorFallbacks';
import { useApiErrorHandling } from '../../hooks/useErrorHandling';
import {
  useProxyRules,
  useCreateProxyRule,
  useUpdateProxyRule,
  useDeleteProxyRule,
  useProxyInspect,
} from '../../hooks/useApi';
import type { ProxyRule, ProxyRuleRequest } from '../../services/api';
import {
  Eye,
  Plus,
  Edit,
  Trash2,
  RefreshCw,
  Filter,
  Code,
  Settings,
  ArrowRight,
  CheckCircle2,
  XCircle,
} from 'lucide-react';
import { cn } from '../../utils/cn';

interface ProxyRuleFormData {
  pattern: string;
  type: 'request' | 'response';
  status_codes: number[];
  body_transforms: Array<{
    path: string;
    replace: string;
    operation: 'replace' | 'add' | 'remove';
  }>;
  enabled: boolean;
}

export function ProxyInspector() {
  const [activeTab, setActiveTab] = useState<'rules' | 'inspect'>('rules');
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [editingRule, setEditingRule] = useState<ProxyRule | null>(null);
  const [filterType, setFilterType] = useState<'all' | 'request' | 'response'>('all');
  const [searchPattern, setSearchPattern] = useState('');

  const { handleApiError, retry, clearError, errorState, canRetry } = useApiErrorHandling();

  // Fetch proxy rules
  const {
    data: rulesData,
    isLoading: rulesLoading,
    error: rulesError,
    refetch: refetchRules,
  } = useProxyRules();

  // Fetch intercepted traffic
  const {
    data: inspectData,
    isLoading: inspectLoading,
    error: inspectError,
    refetch: refetchInspect,
  } = useProxyInspect(50);

  // Mutations
  const createRuleMutation = useCreateProxyRule();
  const updateRuleMutation = useUpdateProxyRule();
  const deleteRuleMutation = useDeleteProxyRule();

  // Handle errors
  React.useEffect(() => {
    if (rulesError) {
      handleApiError(rulesError, 'fetch_proxy_rules');
    } else {
      clearError();
    }
  }, [rulesError, handleApiError, clearError]);

  // Filter rules
  const filteredRules = useMemo(() => {
    if (!rulesData?.rules) return [];

    let filtered = rulesData.rules;

    // Filter by type
    if (filterType !== 'all') {
      filtered = filtered.filter((rule) => rule.type === filterType);
    }

    // Filter by search pattern
    if (searchPattern) {
      const searchLower = searchPattern.toLowerCase();
      filtered = filtered.filter(
        (rule) =>
          rule.pattern.toLowerCase().includes(searchLower) ||
          rule.body_transforms.some((t) =>
            t.path.toLowerCase().includes(searchLower) || t.replace.toLowerCase().includes(searchLower)
          )
      );
    }

    return filtered;
  }, [rulesData, filterType, searchPattern]);

  // Handle create rule
  const handleCreateRule = async (formData: ProxyRuleFormData) => {
    try {
      const ruleRequest: ProxyRuleRequest = {
        pattern: formData.pattern,
        type: formData.type,
        status_codes: formData.status_codes,
        body_transforms: formData.body_transforms.map((t) => ({
          path: t.path,
          replace: t.replace,
          operation: t.operation || 'replace',
        })),
        enabled: formData.enabled,
      };

      await createRuleMutation.mutateAsync(ruleRequest);
      setShowCreateForm(false);
      // Reset form would go here if we had a form component
    } catch (error) {
      handleApiError(error, 'create_proxy_rule');
    }
  };

  // Handle update rule
  const handleUpdateRule = async (id: number, formData: ProxyRuleFormData) => {
    try {
      const ruleRequest: ProxyRuleRequest = {
        pattern: formData.pattern,
        type: formData.type,
        status_codes: formData.status_codes,
        body_transforms: formData.body_transforms.map((t) => ({
          path: t.path,
          replace: t.replace,
          operation: t.operation || 'replace',
        })),
        enabled: formData.enabled,
      };

      await updateRuleMutation.mutateAsync({ id, rule: ruleRequest });
      setEditingRule(null);
    } catch (error) {
      handleApiError(error, 'update_proxy_rule');
    }
  };

  // Handle delete rule
  const handleDeleteRule = async (id: number) => {
    if (!confirm('Are you sure you want to delete this proxy replacement rule?')) {
      return;
    }

    try {
      await deleteRuleMutation.mutateAsync(id);
    } catch (error) {
      handleApiError(error, 'delete_proxy_rule');
    }
  };

  // Rules table columns
  const rulesColumns: ResponsiveTableColumn<ProxyRule>[] = [
    {
      header: 'Pattern',
      accessor: 'pattern',
      cell: (rule) => (
        <div className="flex items-center gap-2">
          <code className="text-xs bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded">
            {rule.pattern}
          </code>
        </div>
      ),
    },
    {
      header: 'Type',
      accessor: 'type',
      cell: (rule) => (
        <Badge
          variant={rule.type === 'request' ? 'info' : 'success'}
          className="text-xs"
        >
          {rule.type}
        </Badge>
      ),
    },
    {
      header: 'Transforms',
      accessor: 'body_transforms',
      cell: (rule) => (
        <div className="flex flex-col gap-1">
          {rule.body_transforms.map((transform, idx) => (
            <div key={idx} className="text-xs text-gray-600 dark:text-gray-400">
              <code className="text-xs">{transform.path}</code>
              <ArrowRight className="inline mx-1 h-3 w-3" />
              <span className="text-xs">{transform.replace.substring(0, 30)}...</span>
            </div>
          ))}
        </div>
      ),
    },
    {
      header: 'Status',
      accessor: 'enabled',
      cell: (rule) => (
        <div className="flex items-center gap-2">
          {rule.enabled ? (
            <CheckCircle2 className="h-4 w-4 text-green-600" />
          ) : (
            <XCircle className="h-4 w-4 text-gray-400" />
          )}
          <span className="text-xs">{rule.enabled ? 'Enabled' : 'Disabled'}</span>
        </div>
      ),
    },
    {
      header: 'Actions',
      accessor: 'id',
      cell: (rule) => (
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setEditingRule(rule)}
            className="h-8 w-8 p-0"
          >
            <Edit className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => handleDeleteRule(rule.id)}
            className="h-8 w-8 p-0 text-red-600 hover:text-red-700"
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Proxy Inspector
          </h2>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            Inspect and replace requests/responses from browser proxy mode
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              refetchRules();
              refetchInspect();
            }}
          >
            <RefreshCw className="h-4 w-4 mr-2" />
            Refresh
          </Button>
        </div>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200 dark:border-gray-700">
        <nav className="flex space-x-8">
          <button
            onClick={() => setActiveTab('rules')}
            className={cn(
              'py-4 px-1 border-b-2 font-medium text-sm transition-colors',
              activeTab === 'rules'
                ? 'border-brand-500 text-brand-600 dark:text-brand-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300'
            )}
          >
            <Settings className="inline h-4 w-4 mr-2" />
            Replacement Rules
          </button>
          <button
            onClick={() => setActiveTab('inspect')}
            className={cn(
              'py-4 px-1 border-b-2 font-medium text-sm transition-colors',
              activeTab === 'inspect'
                ? 'border-brand-500 text-brand-600 dark:text-brand-400'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300'
            )}
          >
            <Eye className="inline h-4 w-4 mr-2" />
            Intercepted Traffic
          </button>
        </nav>
      </div>

      {/* Error Display */}
      {errorState.hasError && (
        <DataErrorFallback
          error={errorState.error}
          retry={canRetry ? retry : undefined}
          onDismiss={clearError}
        />
      )}

      {/* Rules Tab */}
      {activeTab === 'rules' && (
        <div className="space-y-4">
          {/* Filters and Actions */}
          <Card>
            <div className="flex flex-col sm:flex-row gap-4 items-start sm:items-center justify-between">
              <div className="flex flex-1 gap-2 items-center">
                <Filter className="h-4 w-4 text-gray-400" />
                <Input
                  placeholder="Search patterns or transforms..."
                  value={searchPattern}
                  onChange={(e) => setSearchPattern(e.target.value)}
                  className="max-w-sm"
                />
                <select
                  value={filterType}
                  onChange={(e) => setFilterType(e.target.value as 'all' | 'request' | 'response')}
                  className="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-sm"
                >
                  <option value="all">All Types</option>
                  <option value="request">Request Rules</option>
                  <option value="response">Response Rules</option>
                </select>
              </div>
              <Button onClick={() => setShowCreateForm(true)}>
                <Plus className="h-4 w-4 mr-2" />
                Create Rule
              </Button>
            </div>
          </Card>

          {/* Rules Table */}
          <Card>
            {rulesLoading ? (
              <SkeletonTable columns={5} rows={5} />
            ) : rulesError ? (
              <div className="p-8 text-center text-red-600">
                Failed to load proxy rules. {canRetry && <Button onClick={retry}>Retry</Button>}
              </div>
            ) : filteredRules.length === 0 ? (
              <div className="p-8 text-center text-gray-500">
                {rulesData?.rules.length === 0
                  ? 'No proxy replacement rules configured. Create one to get started.'
                  : 'No rules match your filters.'}
              </div>
            ) : (
              <ResponsiveTable
                data={filteredRules}
                columns={rulesColumns}
                keyExtractor={(rule) => rule.id.toString()}
              />
            )}
          </Card>

          {/* Create/Edit Form Modal */}
          {(showCreateForm || editingRule) && (
            <ProxyRuleForm
              rule={editingRule}
              onSave={(formData) => {
                if (editingRule) {
                  handleUpdateRule(editingRule.id, formData);
                } else {
                  handleCreateRule(formData);
                }
              }}
              onCancel={() => {
                setShowCreateForm(false);
                setEditingRule(null);
              }}
            />
          )}
        </div>
      )}

      {/* Inspect Tab */}
      {activeTab === 'inspect' && (
        <div className="space-y-4">
          <Card>
            <div className="space-y-4">
              {inspectLoading ? (
                <div className="p-8 text-center">
                  <RefreshCw className="h-6 w-6 animate-spin mx-auto mb-2 text-gray-400" />
                  <p className="text-sm text-gray-500">Loading intercepted traffic...</p>
                </div>
              ) : inspectError ? (
                <div className="p-8 text-center text-red-600">
                  Failed to load intercepted traffic. {canRetry && <Button onClick={retry}>Retry</Button>}
                </div>
              ) : inspectData?.message ? (
                <div className="p-8 text-center">
                  <Code className="h-12 w-12 mx-auto mb-4 text-gray-400" />
                  <p className="text-gray-600 dark:text-gray-400">{inspectData.message}</p>
                  <p className="text-sm text-gray-500 mt-2">
                    Request/response inspection will be available in a future version.
                  </p>
                </div>
              ) : (
                <div className="space-y-4">
                  <div>
                    <h3 className="text-lg font-semibold mb-2">Intercepted Requests</h3>
                    {inspectData?.requests.length === 0 ? (
                      <p className="text-sm text-gray-500">No requests intercepted yet.</p>
                    ) : (
                      <div className="space-y-2">
                        {inspectData?.requests.map((req) => (
                          <div
                            key={req.id}
                            className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg"
                          >
                            <div className="flex items-center gap-2 mb-2">
                              <Badge variant="info">{req.method}</Badge>
                              <code className="text-sm">{req.url}</code>
                              <span className="text-xs text-gray-500">{req.timestamp}</span>
                            </div>
                            {req.body && (
                              <pre className="text-xs bg-gray-50 dark:bg-gray-900 p-2 rounded mt-2 overflow-x-auto">
                                {req.body}
                              </pre>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                  <div>
                    <h3 className="text-lg font-semibold mb-2">Intercepted Responses</h3>
                    {inspectData?.responses.length === 0 ? (
                      <p className="text-sm text-gray-500">No responses intercepted yet.</p>
                    ) : (
                      <div className="space-y-2">
                        {inspectData?.responses.map((res) => (
                          <div
                            key={res.id}
                            className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg"
                          >
                            <div className="flex items-center gap-2 mb-2">
                              <Badge
                                variant={
                                  res.status_code >= 200 && res.status_code < 300
                                    ? 'success'
                                    : res.status_code >= 400 && res.status_code < 500
                                      ? 'warning'
                                      : 'danger'
                                }
                              >
                                {res.status_code}
                              </Badge>
                              <span className="text-xs text-gray-500">{res.timestamp}</span>
                            </div>
                            {res.body && (
                              <pre className="text-xs bg-gray-50 dark:bg-gray-900 p-2 rounded mt-2 overflow-x-auto">
                                {res.body}
                              </pre>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}

/**
 * Proxy Rule Form Component
 * Simple form for creating/editing proxy replacement rules
 */
interface ProxyRuleFormProps {
  rule?: ProxyRule | null;
  onSave: (formData: ProxyRuleFormData) => void;
  onCancel: () => void;
}

function ProxyRuleForm({ rule, onSave, onCancel }: ProxyRuleFormProps) {
  const [formData, setFormData] = useState<ProxyRuleFormData>({
    pattern: rule?.pattern || '',
    type: rule?.type || 'request',
    status_codes: rule?.status_codes || [],
    body_transforms: rule?.body_transforms || [{ path: '', replace: '', operation: 'replace' }],
    enabled: rule?.enabled ?? true,
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSave(formData);
  };

  const addTransform = () => {
    setFormData({
      ...formData,
      body_transforms: [
        ...formData.body_transforms,
        { path: '', replace: '', operation: 'replace' },
      ],
    });
  };

  const removeTransform = (index: number) => {
    setFormData({
      ...formData,
      body_transforms: formData.body_transforms.filter((_, i) => i !== index),
    });
  };

  const updateTransform = (index: number, field: string, value: string) => {
    const updated = [...formData.body_transforms];
    updated[index] = { ...updated[index], [field]: value };
    setFormData({ ...formData, body_transforms: updated });
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <Card className="w-full max-w-2xl max-h-[90vh] overflow-y-auto m-4">
        <div className="p-6">
          <h3 className="text-xl font-semibold mb-4">
            {rule ? 'Edit Proxy Rule' : 'Create Proxy Rule'}
          </h3>

          <form onSubmit={handleSubmit} className="space-y-4">
            {/* Pattern */}
            <div>
              <label className="block text-sm font-medium mb-1">URL Pattern</label>
              <Input
                value={formData.pattern}
                onChange={(e) => setFormData({ ...formData, pattern: e.target.value })}
                placeholder="/api/users/*"
                required
              />
              <p className="text-xs text-gray-500 mt-1">
                Supports wildcards (e.g., /api/users/*)
              </p>
            </div>

            {/* Type */}
            <div>
              <label className="block text-sm font-medium mb-1">Rule Type</label>
              <select
                value={formData.type}
                onChange={(e) =>
                  setFormData({ ...formData, type: e.target.value as 'request' | 'response' })
                }
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800"
              >
                <option value="request">Request</option>
                <option value="response">Response</option>
              </select>
            </div>

            {/* Status Codes (for response rules) */}
            {formData.type === 'response' && (
              <div>
                <label className="block text-sm font-medium mb-1">Status Codes (comma-separated)</label>
                <Input
                  value={formData.status_codes.join(', ')}
                  onChange={(e) => {
                    const codes = e.target.value
                      .split(',')
                      .map((s) => parseInt(s.trim()))
                      .filter((n) => !isNaN(n));
                    setFormData({ ...formData, status_codes: codes });
                  }}
                  placeholder="200, 201, 404"
                />
              </div>
            )}

            {/* Body Transforms */}
            <div>
              <label className="block text-sm font-medium mb-2">Body Transformations</label>
              <div className="space-y-3">
                {formData.body_transforms.map((transform, index) => (
                  <div key={index} className="p-3 border border-gray-200 dark:border-gray-700 rounded-lg space-y-2">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-sm font-medium">Transform {index + 1}</span>
                      {formData.body_transforms.length > 1 && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          onClick={() => removeTransform(index)}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      )}
                    </div>
                    <div>
                      <label className="block text-xs font-medium mb-1">JSONPath</label>
                      <Input
                        value={transform.path}
                        onChange={(e) => updateTransform(index, 'path', e.target.value)}
                        placeholder="$.userId"
                        required
                      />
                    </div>
                    <div>
                      <label className="block text-xs font-medium mb-1">Replacement Value</label>
                      <Input
                        value={transform.replace}
                        onChange={(e) => updateTransform(index, 'replace', e.target.value)}
                        placeholder="{{uuid}}"
                        required
                      />
                      <p className="text-xs text-gray-500 mt-1">
                        Supports templates: {'{{'}uuid{'}}'}, {'{{'}faker.email{'}}'}, etc.
                      </p>
                    </div>
                    <div>
                      <label className="block text-xs font-medium mb-1">Operation</label>
                      <select
                        value={transform.operation}
                        onChange={(e) =>
                          updateTransform(index, 'operation', e.target.value)
                        }
                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-sm"
                      >
                        <option value="replace">Replace</option>
                        <option value="add">Add</option>
                        <option value="remove">Remove</option>
                      </select>
                    </div>
                  </div>
                ))}
                <Button type="button" variant="outline" onClick={addTransform} className="w-full">
                  <Plus className="h-4 w-4 mr-2" />
                  Add Transformation
                </Button>
              </div>
            </div>

            {/* Enabled */}
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="enabled"
                checked={formData.enabled}
                onChange={(e) => setFormData({ ...formData, enabled: e.target.checked })}
                className="h-4 w-4"
              />
              <label htmlFor="enabled" className="text-sm font-medium">
                Enabled
              </label>
            </div>

            {/* Actions */}
            <div className="flex justify-end gap-2 pt-4 border-t">
              <Button type="button" variant="outline" onClick={onCancel}>
                Cancel
              </Button>
              <Button type="submit">{rule ? 'Update' : 'Create'} Rule</Button>
            </div>
          </form>
        </div>
      </Card>
    </div>
  );
}
