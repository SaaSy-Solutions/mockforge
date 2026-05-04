import { logger } from '@/utils/logger';
import React from 'react';
import { ChevronDown } from 'lucide-react';
import { useEnvironments, useSetActiveEnvironment } from '../../hooks/useApi';
import { Button } from '../ui/button';
import type { EnvironmentSummary } from '../../types';
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

  const activeEnvironment = (environments?.environments as EnvironmentSummary[] | undefined)?.find((env: EnvironmentSummary) => env.active) as EnvironmentSummary | undefined;

  const handleEnvironmentSwitch = async (environment: EnvironmentSummary) => {
    try {
      const envId = environment.is_global ? 'global' : environment.id;
      await setActiveEnvironment.mutateAsync(envId);
      toast.success(`Switched to "${environment.name}" environment`);
    } catch {
      toast.error('Failed to switch environment');
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 rounded-md bg-muted">
        <div className="animate-pulse">
          <div className="w-16 h-4 bg-muted rounded"></div>
        </div>
      </div>
    );
  }

  if (!activeEnvironment) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 rounded-md bg-muted text-muted-foreground">
        <span className="text-sm">No Environment</span>
      </div>
    );
  }

  const availableEnvironments = (environments?.environments as EnvironmentSummary[] | undefined)?.filter((env: EnvironmentSummary) => !env.active) || [];

  if (compact) {
    return (
      <div className="flex items-center gap-1">
        {activeEnvironment.color && (
          <div
            className="w-2 h-2 rounded-full"
            style={{ backgroundColor: activeEnvironment.color.hex }}
          />
        )}
        <span className="text-xs text-muted-foreground">
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
          className="flex items-center gap-2 px-3 py-2 rounded-md hover:bg-accent hover:text-accent-foreground"
        >
          {activeEnvironment.color && (
            <div
              className="w-3 h-3 rounded-full border border-white shadow-sm"
              style={{ backgroundColor: activeEnvironment.color.hex }}
            />
          )}
          <span className="text-sm font-medium text-foreground">
            {activeEnvironment.name}
          </span>
          {activeEnvironment.is_global && (
            <span className="text-xs text-muted-foreground">(Global)</span>
          )}
          <ChevronDown className="w-4 h-4 text-muted-foreground" />
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
            <span className="text-xs text-muted-foreground">(Global)</span>
          )}
        </DropdownMenuItem>

        {availableEnvironments.length > 0 && (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuLabel>Switch Environment</DropdownMenuLabel>
            {(availableEnvironments as EnvironmentSummary[]).map((environment: EnvironmentSummary) => (
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
                  <span className="text-xs text-muted-foreground">(Global)</span>
                )}
              </DropdownMenuItem>
            ))}
          </>
        )}

        <DropdownMenuSeparator />
        <DropdownMenuItem disabled className="text-xs text-muted-foreground">
          {availableEnvironments.length} environment{availableEnvironments.length !== 1 ? 's' : ''} available
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
