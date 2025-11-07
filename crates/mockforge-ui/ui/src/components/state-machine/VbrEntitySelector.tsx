//! VBR Entity Selector Component
//!
//! Allows users to select VBR entities as resources for state machines.
//! Displays available entities and their state machine status.

import React, { useEffect, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Badge } from '../ui/Badge';
import { X, Search, CheckCircle2, Circle, Loader2 } from 'lucide-react';
import { apiService } from '../../services/api';
import { logger } from '@/utils/logger';
import { cn } from '@/utils/cn';

interface VbrEntitySelectorProps {
  selectedEntity?: string;
  onSelect: (entityName: string) => void;
  onClose: () => void;
}

interface VbrEntity {
  name: string;
  table_name: string;
  has_state_machine: boolean;
  state_machine_resource_type?: string;
  fields: Array<{
    name: string;
    type: string;
  }>;
}

export function VbrEntitySelector({
  selectedEntity,
  onSelect,
  onClose,
}: VbrEntitySelectorProps) {
  const [entities, setEntities] = useState<VbrEntity[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [selected, setSelected] = useState<string | undefined>(selectedEntity);

  // Load entities
  useEffect(() => {
    loadEntities();
  }, []);

  const loadEntities = async () => {
    try {
      setLoading(true);
      setError(null);

      // Try to get entities from VBR API
      // Since there's no direct endpoint to list all entities, we'll try common entity names
      // or we can add a management endpoint later
      // For now, we'll use a workaround: try to get state machines and infer entities
      const stateMachinesResponse = await apiService.getStateMachines();

      // Map state machines to potential entities
      const entityMap = new Map<string, VbrEntity>();

      // Add entities from state machines
      stateMachinesResponse.state_machines.forEach((sm) => {
        entityMap.set(sm.resource_type, {
          name: sm.resource_type,
          table_name: sm.resource_type.toLowerCase() + 's',
          has_state_machine: true,
          state_machine_resource_type: sm.resource_type,
          fields: [],
        });
      });

      // Try to fetch some common entity types from VBR API
      const commonEntities = ['User', 'Order', 'Product', 'Cart', 'Payment', 'Session'];
      for (const entityName of commonEntities) {
        if (!entityMap.has(entityName)) {
          try {
            // Try to access the entity endpoint to see if it exists
            // This is a heuristic - in production, we'd have a proper endpoint
            const response = await fetch(`/vbr-api/${entityName}?limit=1`);
            if (response.ok) {
              entityMap.set(entityName, {
                name: entityName,
                table_name: entityName.toLowerCase() + 's',
                has_state_machine: false,
                fields: [],
              });
            }
          } catch (err) {
            // Entity doesn't exist or endpoint not available
            continue;
          }
        }
      }

      setEntities(Array.from(entityMap.values()));
    } catch (err) {
      logger.error('Failed to load VBR entities', err);
      setError(err instanceof Error ? err.message : 'Failed to load entities');
    } finally {
      setLoading(false);
    }
  };

  const filteredEntities = entities.filter((entity) =>
    entity.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    entity.table_name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleSelect = (entityName: string) => {
    setSelected(entityName);
    onSelect(entityName);
  };

  return (
    <Card className="w-full max-w-2xl">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">Select VBR Entity</CardTitle>
        <Button
          onClick={onClose}
          size="sm"
          variant="ghost"
          className="h-6 w-6 p-0"
        >
          <X className="h-4 w-4" />
        </Button>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Search */}
        <div className="relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
          <Input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search entities..."
            className="pl-10"
          />
        </div>

        {/* Loading state */}
        {loading && (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            <span className="ml-2 text-sm text-gray-500">Loading entities...</span>
          </div>
        )}

        {/* Error state */}
        {error && (
          <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
            <p className="text-red-800 dark:text-red-200 text-sm">{error}</p>
          </div>
        )}

        {/* Entity list */}
        {!loading && !error && (
          <div className="space-y-2 max-h-96 overflow-y-auto">
            {filteredEntities.length === 0 ? (
              <div className="text-center py-8 text-sm text-gray-500">
                {searchQuery ? 'No entities found matching your search' : 'No entities available'}
              </div>
            ) : (
              filteredEntities.map((entity) => (
                <div
                  key={entity.name}
                  onClick={() => handleSelect(entity.name)}
                  className={cn(
                    'p-3 border rounded-lg cursor-pointer transition-colors',
                    selected === entity.name
                      ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                      : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
                  )}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      {selected === entity.name ? (
                        <CheckCircle2 className="h-5 w-5 text-blue-500" />
                      ) : (
                        <Circle className="h-5 w-5 text-gray-400" />
                      )}
                      <div>
                        <div className="font-medium text-sm">{entity.name}</div>
                        <div className="text-xs text-gray-500 dark:text-gray-400">
                          Table: {entity.table_name}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      {entity.has_state_machine && (
                        <Badge variant="success" className="text-xs">
                          Has State Machine
                        </Badge>
                      )}
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-2 pt-4 border-t">
          <Button onClick={onClose} variant="outline" size="sm">
            Cancel
          </Button>
          <Button
            onClick={() => {
              if (selected) {
                onSelect(selected);
              }
            }}
            variant="default"
            size="sm"
            disabled={!selected}
          >
            Select
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
