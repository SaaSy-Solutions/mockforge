/**
 * Federation Form Component
 *
 * Form for creating and editing federations
 */

import React, { useState, useEffect } from 'react';
import { useCreateFederation, useUpdateFederation, Federation, FederationService } from '../../hooks/useFederation';
import { Card } from '../ui/Card';
import { ArrowLeft, Save, Plus, Trash2 } from 'lucide-react';

export interface FederationFormProps {
  federation?: Federation;
  orgId: string;
  onSave?: () => void;
  onCancel?: () => void;
}

export const FederationForm: React.FC<FederationFormProps> = ({
  federation,
  orgId,
  onSave,
  onCancel,
}) => {
  const [name, setName] = useState(federation?.name || '');
  const [description, setDescription] = useState(federation?.description || '');
  const [services, setServices] = useState<FederationService[]>(
    federation?.services || []
  );

  const createFederation = useCreateFederation();
  const updateFederation = useUpdateFederation();

  const addService = () => {
    setServices([
      ...services,
      {
        name: '',
        workspace_id: '',
        base_path: '',
        reality_level: 'mock_v3',
        config: {},
        dependencies: [],
      },
    ]);
  };

  const removeService = (index: number) => {
    setServices(services.filter((_, i) => i !== index));
  };

  const updateService = (index: number, field: keyof FederationService, value: any) => {
    const updated = [...services];
    (updated[index] as any)[field] = value;
    setServices(updated);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    try {
      if (federation) {
        await updateFederation.mutateAsync({
          id: federation.id,
          data: { name, description, services },
        });
      } else {
        await createFederation.mutateAsync({
          name,
          description,
          org_id: orgId,
          services,
        });
      }

      onSave?.();
    } catch (err) {
      alert(`Failed to save federation: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        {onCancel && (
          <button
            onClick={onCancel}
            className="p-2 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
          >
            <ArrowLeft className="h-5 w-5" />
          </button>
        )}
        <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
          {federation ? 'Edit Federation' : 'Create Federation'}
        </h1>
      </div>

      <form onSubmit={handleSubmit} className="space-y-6">
        <Card className="p-6">
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Federation Name
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Description
              </label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                rows={3}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
              />
            </div>
          </div>
        </Card>

        <Card className="p-6">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
              Services ({services.length})
            </h3>
            <button
              type="button"
              onClick={addService}
              className="flex items-center gap-2 px-3 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors text-sm"
            >
              <Plus className="h-4 w-4" />
              Add Service
            </button>
          </div>

          {services.length === 0 ? (
            <p className="text-sm text-gray-600 dark:text-gray-400 text-center py-4">
              No services configured. Click "Add Service" to add one.
            </p>
          ) : (
            <div className="space-y-4">
              {services.map((service, index) => (
                <div
                  key={index}
                  className="p-4 bg-gray-50 dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-700"
                >
                  <div className="flex items-center justify-between mb-4">
                    <h4 className="font-medium text-gray-900 dark:text-white">
                      Service {index + 1}
                    </h4>
                    <button
                      type="button"
                      onClick={() => removeService(index)}
                      className="p-1 text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300"
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                      <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                        Service Name
                      </label>
                      <input
                        type="text"
                        value={service.name}
                        onChange={(e) => updateService(index, 'name', e.target.value)}
                        required
                        placeholder="auth"
                        className="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                        Workspace ID
                      </label>
                      <input
                        type="text"
                        value={service.workspace_id}
                        onChange={(e) => updateService(index, 'workspace_id', e.target.value)}
                        required
                        placeholder="550e8400-e29b-41d4-a716-446655440000"
                        className="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white font-mono"
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                        Base Path
                      </label>
                      <input
                        type="text"
                        value={service.base_path}
                        onChange={(e) => updateService(index, 'base_path', e.target.value)}
                        required
                        placeholder="/auth"
                        className="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                      />
                    </div>

                    <div>
                      <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                        Reality Level
                      </label>
                      <select
                        value={service.reality_level}
                        onChange={(e) => updateService(index, 'reality_level', e.target.value)}
                        className="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                      >
                        <option value="real">Real</option>
                        <option value="mock_v3">Mock V3</option>
                        <option value="blended">Blended</option>
                        <option value="chaos_driven">Chaos Driven</option>
                      </select>
                    </div>
                  </div>

                  <div className="mt-4">
                    <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                      Dependencies (comma-separated service names)
                    </label>
                    <input
                      type="text"
                      value={service.dependencies?.join(', ') || ''}
                      onChange={(e) =>
                        updateService(
                          index,
                          'dependencies',
                          e.target.value.split(',').map((s) => s.trim()).filter(Boolean)
                        )
                      }
                      placeholder="auth, payments"
                      className="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>

        <div className="flex items-center justify-end gap-4">
          {onCancel && (
            <button
              type="button"
              onClick={onCancel}
              className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
            >
              Cancel
            </button>
          )}
          <button
            type="submit"
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          >
            <Save className="h-4 w-4" />
            Save Federation
          </button>
        </div>
      </form>
    </div>
  );
};
