//! Sub-Scenario Editor Component
//!
//! Allows users to create and configure nested sub-scenarios within state machines.
//! Supports input/output mapping and sub-scenario selection.

import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Badge } from '../ui/Badge';
import { X, Plus, Trash2, ArrowRight, Loader2 } from 'lucide-react';
import { apiService } from '../../services/api';
import { logger } from '@/utils/logger';
import { cn } from '@/utils/cn';

interface SubScenarioEditorProps {
  subScenarioId?: string;
  onSave: (subScenario: SubScenarioConfig) => void;
  onCancel: () => void;
}

interface SubScenarioConfig {
  id: string;
  name: string;
  description?: string;
  state_machine_resource_type: string;
  input_mapping: Record<string, string>;
  output_mapping: Record<string, string>;
}

interface AvailableSubScenario {
  resource_type: string;
  state_count: number;
  transition_count: number;
}

export function SubScenarioEditor({
  subScenarioId,
  onSave,
  onCancel,
}: SubScenarioEditorProps) {
  const [availableSubScenarios, setAvailableSubScenarios] = useState<
    AvailableSubScenario[]
  >([]);
  const [loading, setLoading] = useState(true);
  const [selectedResourceType, setSelectedResourceType] = useState<string>('');
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [inputMapping, setInputMapping] = useState<Array<{ key: string; value: string }>>([
    { key: '', value: '' },
  ]);
  const [outputMapping, setOutputMapping] = useState<Array<{ key: string; value: string }>>([
    { key: '', value: '' },
  ]);

  useEffect(() => {
    loadAvailableSubScenarios();
    if (subScenarioId) {
      // Load existing sub-scenario if editing
      loadSubScenario(subScenarioId);
    }
  }, [subScenarioId]);

  const loadAvailableSubScenarios = async () => {
    try {
      setLoading(true);
      const response = await apiService.getStateMachines();
      setAvailableSubScenarios(response.state_machines);
    } catch (err) {
      logger.error('Failed to load available sub-scenarios', err);
    } finally {
      setLoading(false);
    }
  };

  const loadSubScenario = async (id: string) => {
    // In a real implementation, we'd fetch the sub-scenario by ID
    // For now, we'll use the resource type to infer the sub-scenario
    try {
      // This would be a separate API call to get sub-scenario details
      // For now, we'll just set the ID
      setName(id);
      setSelectedResourceType(id);
    } catch (err) {
      logger.error('Failed to load sub-scenario', err);
    }
  };

  const handleAddInputMapping = () => {
    setInputMapping([...inputMapping, { key: '', value: '' }]);
  };

  const handleRemoveInputMapping = (index: number) => {
    setInputMapping(inputMapping.filter((_, i) => i !== index));
  };

  const handleUpdateInputMapping = (
    index: number,
    field: 'key' | 'value',
    value: string
  ) => {
    setInputMapping(
      inputMapping.map((m, i) => (i === index ? { ...m, [field]: value } : m))
    );
  };

  const handleAddOutputMapping = () => {
    setOutputMapping([...outputMapping, { key: '', value: '' }]);
  };

  const handleRemoveOutputMapping = (index: number) => {
    setOutputMapping(outputMapping.filter((_, i) => i !== index));
  };

  const handleUpdateOutputMapping = (
    index: number,
    field: 'key' | 'value',
    value: string
  ) => {
    setOutputMapping(
      outputMapping.map((m, i) => (i === index ? { ...m, [field]: value } : m))
    );
  };

  const handleSave = () => {
    if (!selectedResourceType || !name) {
      return;
    }

    const config: SubScenarioConfig = {
      id: subScenarioId || `sub-scenario-${Date.now()}`,
      name,
      description: description || undefined,
      state_machine_resource_type: selectedResourceType,
      input_mapping: inputMapping
        .filter((m) => m.key && m.value)
        .reduce((acc, m) => {
          acc[m.key] = m.value;
          return acc;
        }, {} as Record<string, string>),
      output_mapping: outputMapping
        .filter((m) => m.key && m.value)
        .reduce((acc, m) => {
          acc[m.key] = m.value;
          return acc;
        }, {} as Record<string, string>),
    };

    onSave(config);
  };

  return (
    <Card className="w-full max-w-3xl max-h-[90vh] overflow-hidden flex flex-col">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">
          {subScenarioId ? 'Edit Sub-Scenario' : 'Create Sub-Scenario'}
        </CardTitle>
        <Button
          onClick={onCancel}
          size="sm"
          variant="ghost"
          className="h-6 w-6 p-0"
        >
          <X className="h-4 w-4" />
        </Button>
      </CardHeader>
      <CardContent className="flex-1 overflow-y-auto space-y-4">
        {/* Basic Info */}
        <div className="space-y-2">
          <label className="text-sm font-medium">Name *</label>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Sub-scenario name"
          />
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Description</label>
          <Input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Optional description"
          />
        </div>

        {/* Sub-Scenario Selection */}
        <div className="space-y-2">
          <label className="text-sm font-medium">State Machine *</label>
          {loading ? (
            <div className="flex items-center gap-2 py-2">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span className="text-sm text-gray-500">Loading state machines...</span>
            </div>
          ) : (
            <select
              value={selectedResourceType}
              onChange={(e) => setSelectedResourceType(e.target.value)}
              className="w-full px-3 py-2 border rounded-md bg-white dark:bg-gray-800"
            >
              <option value="">Select a state machine...</option>
              {availableSubScenarios.map((sm) => (
                <option key={sm.resource_type} value={sm.resource_type}>
                  {sm.resource_type} ({sm.state_count} states, {sm.transition_count} transitions)
                </option>
              ))}
            </select>
          )}
        </div>

        {/* Input Mapping */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium">Input Mapping</label>
            <Button
              onClick={handleAddInputMapping}
              size="sm"
              variant="outline"
              className="h-7"
            >
              <Plus className="h-3 w-3 mr-1" />
              Add
            </Button>
          </div>
          <div className="space-y-2">
            {inputMapping.map((mapping, index) => (
              <div key={index} className="flex items-center gap-2">
                <Input
                  value={mapping.key}
                  onChange={(e) =>
                    handleUpdateInputMapping(index, 'key', e.target.value)
                  }
                  placeholder="Parent variable"
                  className="flex-1"
                />
                <ArrowRight className="h-4 w-4 text-gray-400" />
                <Input
                  value={mapping.value}
                  onChange={(e) =>
                    handleUpdateInputMapping(index, 'value', e.target.value)
                  }
                  placeholder="Sub-scenario variable"
                  className="flex-1"
                />
                <Button
                  onClick={() => handleRemoveInputMapping(index)}
                  size="sm"
                  variant="ghost"
                  className="h-8 w-8 p-0"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            ))}
          </div>
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Map parent state machine variables to sub-scenario input variables
          </p>
        </div>

        {/* Output Mapping */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium">Output Mapping</label>
            <Button
              onClick={handleAddOutputMapping}
              size="sm"
              variant="outline"
              className="h-7"
            >
              <Plus className="h-3 w-3 mr-1" />
              Add
            </Button>
          </div>
          <div className="space-y-2">
            {outputMapping.map((mapping, index) => (
              <div key={index} className="flex items-center gap-2">
                <Input
                  value={mapping.key}
                  onChange={(e) =>
                    handleUpdateOutputMapping(index, 'key', e.target.value)
                  }
                  placeholder="Sub-scenario variable"
                  className="flex-1"
                />
                <ArrowRight className="h-4 w-4 text-gray-400" />
                <Input
                  value={mapping.value}
                  onChange={(e) =>
                    handleUpdateOutputMapping(index, 'value', e.target.value)
                  }
                  placeholder="Parent variable"
                  className="flex-1"
                />
                <Button
                  onClick={() => handleRemoveOutputMapping(index)}
                  size="sm"
                  variant="ghost"
                  className="h-8 w-8 p-0"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            ))}
          </div>
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Map sub-scenario output variables back to parent state machine variables
          </p>
        </div>
      </CardContent>

      {/* Actions */}
      <div className="flex justify-end gap-2 p-4 border-t">
        <Button onClick={onCancel} variant="outline" size="sm">
          Cancel
        </Button>
        <Button
          onClick={handleSave}
          variant="default"
          size="sm"
          disabled={!selectedResourceType || !name}
        >
          Save
        </Button>
      </div>
    </Card>
  );
}
