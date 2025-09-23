import React from 'react';
import { ChevronDown } from 'lucide-react';
import { useEnvironments, useSetActiveEnvironment } from '../../hooks/useApi';
import { Button } from '../ui/button';
import { EnvironmentSummary } from '../../types';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
  DropdownMenuLabel,
} from '../ui/DropdownMenu';
import { toast } from '../ui/Toast';

interface EnvironmentIndicatorProps {
  workspaceId: string;
  compact?: boolean;
}

export function EnvironmentIndicator({ workspaceId, compact = false }: EnvironmentIndicatorProps) {
  const { data: environments, isLoading } = useEnvironments(workspaceId);
  const setActiveEnvironment = useSetActiveEnvironment(workspaceId);

  const activeEnvironment = environments?.environments.find(env => env.active);

  const handleEnvironmentSwitch = async (environment: EnvironmentSummary) => {
    try {
      const envId = environment.is_global ? 'global' : environment.id;
      await setActiveEnvironment.mutateAsync(envId);
      toast.success(`Switched to "${environment.name}" environment`);
    } catch (error) {
      toast.error('Failed to switch environment');
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 rounded-md bg-gray-100 dark:bg-gray-800">
        <div className="animate-pulse">
          <div className="w-16 h-4 bg-gray-300 dark:bg-gray-600 rounded"></div>
        </div>
      </div>
    );
  }

  if (!activeEnvironment) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 rounded-md bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400">
        <span className="text-sm">No Environment</span>
      </div>
    );
  }

  const availableEnvironments = environments?.environments.filter(env => !env.active) || [];

  if (compact) {
    return (
      <div className="flex items-center gap-1">
        {activeEnvironment.color && (
          <div
            className="w-2 h-2 rounded-full"
            style={{ backgroundColor: activeEnvironment.color.hex }}
          />
        )}
        <span className="text-xs text-gray-600 dark:text-gray-400">
          {activeEnvironment.name}
        </span>
      </div>
    );
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          className="flex items-center gap-2 px-3 py-2 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800"
        >
          {activeEnvironment.color && (
            <div
              className="w-3 h-3 rounded-full border border-white shadow-sm"
              style={{ backgroundColor: activeEnvironment.color.hex }}
            />
          )}
          <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
            {activeEnvironment.name}
          </span>
          {activeEnvironment.is_global && (
            <span className="text-xs text-gray-500 dark:text-gray-400">(Global)</span>
          )}
          <ChevronDown className="w-4 h-4 text-gray-500 dark:text-gray-400" />
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>Active Environment</DropdownMenuLabel>
        <DropdownMenuItem disabled className="flex items-center gap-2">
          {activeEnvironment.color && (
            <div
              className="w-3 h-3 rounded-full border border-white shadow-sm"
              style={{ backgroundColor: activeEnvironment.color.hex }}
            />
          )}
          <span className="font-medium">{activeEnvironment.name}</span>
          {activeEnvironment.is_global && (
            <span className="text-xs text-gray-500 dark:text-gray-400">(Global)</span>
          )}
        </DropdownMenuItem>

        {availableEnvironments.length > 0 && (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuLabel>Switch Environment</DropdownMenuLabel>
            {availableEnvironments.map((environment) => (
              <DropdownMenuItem
                key={environment.id}
                onClick={() => handleEnvironmentSwitch(environment)}
                className="flex items-center gap-2 cursor-pointer"
              >
                {environment.color && (
                  <div
                    className="w-3 h-3 rounded-full border border-white shadow-sm"
                    style={{ backgroundColor: environment.color.hex }}
                  />
                )}
                <span>{environment.name}</span>
                {environment.is_global && (
                  <span className="text-xs text-gray-500 dark:text-gray-400">(Global)</span>
                )}
              </DropdownMenuItem>
            ))}
          </>
        )}

        <DropdownMenuSeparator />
        <DropdownMenuItem disabled className="text-xs text-gray-500 dark:text-gray-400">
          {availableEnvironments.length} environment{availableEnvironments.length !== 1 ? 's' : ''} available
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
