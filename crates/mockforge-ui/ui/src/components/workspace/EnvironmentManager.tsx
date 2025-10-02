import React, { useState } from 'react';
import { Plus, Settings, Trash2, Play, GripVertical } from 'lucide-react';
import { useEnvironments, useCreateEnvironment, useUpdateEnvironment, useDeleteEnvironment, useSetActiveEnvironment, useEnvironmentVariables, useUpdateEnvironmentsOrder } from '../../hooks/useApi';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger, DialogFooter } from '../ui/Dialog';
import { ContextMenu, ContextMenuContent, ContextMenuTrigger } from '../ui/ContextMenu';
import { ModernCard, ContextMenuItem } from '../ui/DesignSystem';
import type { EnvironmentSummary, CreateEnvironmentRequest, UpdateEnvironmentRequest, EnvironmentColor } from '../../types';
import { toast } from '../ui/Toast';

interface EnvironmentManagerProps {
  workspaceId: string;
  onEnvironmentSelect?: (environmentId: string) => void;
}

const PREDEFINED_COLORS = [
  { hex: '#3B82F6', name: 'Blue' },
  { hex: '#EF4444', name: 'Red' },
  { hex: '#10B981', name: 'Green' },
  { hex: '#F59E0B', name: 'Yellow' },
  { hex: '#8B5CF6', name: 'Purple' },
  { hex: '#F97316', name: 'Orange' },
  { hex: '#06B6D4', name: 'Cyan' },
  { hex: '#84CC16', name: 'Lime' },
];

export function EnvironmentManager({ workspaceId, onEnvironmentSelect }: EnvironmentManagerProps) {
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
  const [editingEnvironment, setEditingEnvironment] = useState<EnvironmentSummary | null>(null);
  const [createForm, setCreateForm] = useState<CreateEnvironmentRequest>({ name: '', description: '' });
  const [editForm, setEditForm] = useState<UpdateEnvironmentRequest>({});
  const [selectedColor, setSelectedColor] = useState<EnvironmentColor | null>(null);
  const [draggedItem, setDraggedItem] = useState<string | null>(null);

  const { data: environments, isLoading, error } = useEnvironments(workspaceId);
  const createEnvironment = useCreateEnvironment(workspaceId);
  const updateEnvironment = useUpdateEnvironment(workspaceId, editingEnvironment?.id || '');
  const deleteEnvironment = useDeleteEnvironment(workspaceId);
  const setActiveEnvironment = useSetActiveEnvironment(workspaceId);
  const updateEnvironmentsOrder = useUpdateEnvironmentsOrder(workspaceId);

  const handleCreate = async () => {
    if (!createForm.name.trim()) {
      toast.error('Environment name is required');
      return;
    }

    try {
      await createEnvironment.mutateAsync({
        ...createForm,
        name: createForm.name.trim(),
      });

      toast.success(`Environment "${createForm.name}" created successfully`);
      setCreateForm({ name: '', description: '' });
      setIsCreateDialogOpen(false);
    } catch {
      toast.error('Failed to create environment');
    }
  };

  const handleUpdate = async () => {
    if (!editingEnvironment) return;

    try {
      await updateEnvironment.mutateAsync(editForm);
      toast.success(`Environment "${editingEnvironment.name}" updated successfully`);
      setEditingEnvironment(null);
      setEditForm({});
      setSelectedColor(null);
    } catch {
      toast.error('Failed to update environment');
    }
  };

  const handleDelete = async (environment: EnvironmentSummary) => {
    if (environment.is_global) {
      toast.error('Cannot delete global environment');
      return;
    }

    if (!confirm(`Are you sure you want to delete "${environment.name}"? This action cannot be undone.`)) {
      return;
    }

    try {
      await deleteEnvironment.mutateAsync(environment.id);
      toast.success(`Environment "${environment.name}" deleted successfully`);
    } catch {
      toast.error('Failed to delete environment');
    }
  };

  const handleSetActive = async (environment: EnvironmentSummary) => {
    try {
      const envId = environment.is_global ? 'global' : environment.id;
      await setActiveEnvironment.mutateAsync(envId);
      toast.success(`Switched to "${environment.name}" environment`);
      onEnvironmentSelect?.(environment.id);
    } catch {
      toast.error('Failed to switch environment');
    }
  };

  const handleEdit = (environment: EnvironmentSummary) => {
    setEditingEnvironment(environment);
    setEditForm({
      name: environment.name,
      description: environment.description,
    });
    setSelectedColor(environment.color || null);
  };

  const handleDragStart = (e: React.DragEvent, environmentId: string) => {
    setDraggedItem(environmentId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  };

  const handleDrop = async (e: React.DragEvent, targetEnvironmentId: string) => {
    e.preventDefault();

    if (!draggedItem || draggedItem === targetEnvironmentId) {
      setDraggedItem(null);
      return;
    }

    if (!environments?.environments) {
      setDraggedItem(null);
      return;
    }

    try {
      // Reorder the environments array
      const draggedIndex = environments.environments.findIndex(env => env.id === draggedItem);
      const targetIndex = environments.environments.findIndex(env => env.id === targetEnvironmentId);

      if (draggedIndex === -1 || targetIndex === -1) {
        setDraggedItem(null);
        return;
      }

      const newEnvironments = [...environments.environments];
      const [draggedEnv] = newEnvironments.splice(draggedIndex, 1);
      newEnvironments.splice(targetIndex, 0, draggedEnv);

      // Update the order by sending the new order to the API
      const environmentIds = newEnvironments.map(env => env.id);

      try {
        await updateEnvironmentsOrder.mutateAsync(environmentIds);
        toast.success('Environment order updated');
      } catch {
        toast.error('Failed to update environment order');
        throw error;
      }
    } catch {
      toast.error('Failed to update environment order');
    } finally {
      setDraggedItem(null);
    }
  };

  const EnvironmentCard = ({ environment }: { environment: EnvironmentSummary }) => {
    const { data: variables } = useEnvironmentVariables(workspaceId, environment.id);
    const isDragging = draggedItem === environment.id;

    return (
      <ContextMenu>
        <ContextMenuTrigger>
          <ModernCard
            draggable={!environment.is_global}
            onDragStart={(e) => handleDragStart(e, environment.id)}
            onDragOver={handleDragOver}
            onDrop={(e) => handleDrop(e, environment.id)}
            className={`cursor-pointer transition-all duration-200 hover:shadow-lg ${
              environment.active
                ? 'ring-2 ring-blue-500 bg-blue-50 dark:bg-blue-900/20'
                : 'hover:bg-gray-50 dark:hover:bg-gray-800/50'
            } ${isDragging ? 'opacity-50' : ''} ${!environment.is_global ? 'cursor-move' : ''}`}
            onClick={() => handleSetActive(environment)}
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                {!environment.is_global && (
                  <div className="cursor-move p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded">
                    <GripVertical className="w-4 h-4 text-gray-400" />
                  </div>
                )}
                {environment.color && (
                  <div
                    className="w-4 h-4 rounded-full border-2 border-white shadow-sm"
                    style={{ backgroundColor: environment.color.hex }}
                  />
                )}
                <div>
                  <h3 className="font-medium text-gray-900 dark:text-gray-100">
                    {environment.name}
                    {environment.is_global && (
                      <span className="ml-2 text-xs text-gray-500 dark:text-gray-400">(Global)</span>
                    )}
                  </h3>
                  {environment.description && (
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      {environment.description}
                    </p>
                  )}
                </div>
              </div>

              <div className="flex items-center gap-2">
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  {environment.variable_count} vars
                </span>
                {environment.active && (
                  <div className="w-2 h-2 bg-blue-500 rounded-full" />
                )}
              </div>
            </div>

            {variables && variables.variables.length > 0 && (
              <div className="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
                <div className="flex flex-wrap gap-1">
                  {variables.variables.slice(0, 3).map((variable: unknown) => (
                    <span
                      key={variable.name}
                      className="inline-flex items-center px-2 py-1 rounded-md text-xs font-medium bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200"
                    >
                      {variable.name}
                    </span>
                  ))}
                  {variables.variables.length > 3 && (
                    <span className="text-xs text-gray-500 dark:text-gray-400">
                      +{variables.variables.length - 3} more
                    </span>
                  )}
                </div>
              </div>
            )}
          </ModernCard>
        </ContextMenuTrigger>

        <ContextMenuContent>
          <ContextMenuItem onClick={() => handleSetActive(environment)}>
            <Play className="w-4 h-4 mr-2" />
            Set as Active
          </ContextMenuItem>
          <ContextMenuItem onClick={() => handleEdit(environment)}>
            <Settings className="w-4 h-4 mr-2" />
            Edit Environment
          </ContextMenuItem>
          {!environment.is_global && (
            <ContextMenuItem
              onClick={() => handleDelete(environment)}
              className="text-red-600 dark:text-red-400"
            >
              <Trash2 className="w-4 h-4 mr-2" />
              Delete Environment
            </ContextMenuItem>
          )}
        </ContextMenuContent>
      </ContextMenu>
    );
  };

  if (isLoading) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            Environments
          </h2>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="animate-pulse">
              <div className="h-24 bg-gray-200 dark:bg-gray-700 rounded-lg"></div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            Environments
          </h2>
        </div>
        <div className="text-center py-8 text-red-600 dark:text-red-400">
          Failed to load environments
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Environments
        </h2>

        <Dialog open={isCreateDialogOpen} onOpenChange={setIsCreateDialogOpen}>
          <DialogTrigger asChild>
            <Button className="flex items-center gap-2">
              <Plus className="w-4 h-4" />
              New Environment
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Create New Environment</DialogTitle>
            </DialogHeader>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Name
                </label>
                <Input
                  value={createForm.name}
                  onChange={(e) => setCreateForm(prev => ({ ...prev, name: e.target.value }))}
                  placeholder="e.g., Development, Staging, Production"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Description (Optional)
                </label>
                <Input
                  value={createForm.description || ''}
                  onChange={(e) => setCreateForm(prev => ({ ...prev, description: e.target.value }))}
                  placeholder="Brief description of this environment"
                />
              </div>
            </div>

            <DialogFooter>
              <Button variant="outline" onClick={() => setIsCreateDialogOpen(false)}>
                Cancel
              </Button>
              <Button onClick={handleCreate} disabled={createEnvironment.isPending}>
                {createEnvironment.isPending ? 'Creating...' : 'Create Environment'}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {environments?.environments
          .sort((a: unknown, b: unknown) => {
            // Global environment always first
            if (a.is_global && !b.is_global) return -1;
            if (!a.is_global && b.is_global) return 1;
            // Then sort by order field
            return (a.order || 0) - (b.order || 0);
          })
          .map((environment: unknown) => (
            <EnvironmentCard key={environment.id} environment={environment} />
          ))}
      </div>

      {/* Edit Environment Dialog */}
      <Dialog open={!!editingEnvironment} onOpenChange={(open) => !open && setEditingEnvironment(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Environment</DialogTitle>
          </DialogHeader>

          {editingEnvironment && (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Name
                </label>
                <Input
                  value={editForm.name || ''}
                  onChange={(e) => setEditForm(prev => ({ ...prev, name: e.target.value }))}
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Description (Optional)
                </label>
                <Input
                  value={editForm.description || ''}
                  onChange={(e) => setEditForm(prev => ({ ...prev, description: e.target.value }))}
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Color (Optional)
                </label>
                <div className="flex flex-wrap gap-2">
                  {PREDEFINED_COLORS.map((color) => (
                    <button
                      key={color.hex}
                      onClick={() => setSelectedColor(color)}
                      className={`w-8 h-8 rounded-full border-2 ${
                        selectedColor?.hex === color.hex
                          ? 'border-gray-900 dark:border-gray-100'
                          : 'border-gray-300 dark:border-gray-600'
                      }`}
                      style={{ backgroundColor: color.hex }}
                      title={color.name}
                    />
                  ))}
                </div>
                {selectedColor && (
                  <div className="flex items-center gap-2 mt-2">
                    <div
                      className="w-4 h-4 rounded-full border border-gray-300"
                      style={{ backgroundColor: selectedColor.hex }}
                    />
                    <span className="text-sm text-gray-600 dark:text-gray-400">
                      {selectedColor.name}
                    </span>
                  </div>
                )}
              </div>
            </div>
          )}

          <DialogFooter>
            <Button variant="outline" onClick={() => setEditingEnvironment(null)}>
              Cancel
            </Button>
            <Button onClick={handleUpdate} disabled={updateEnvironment.isPending}>
              {updateEnvironment.isPending ? 'Updating...' : 'Update Environment'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
