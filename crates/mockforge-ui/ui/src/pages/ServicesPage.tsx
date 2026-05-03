import React, { useEffect } from 'react';
import { Search, AlertCircle, RefreshCw } from 'lucide-react';
import { ServicesPanel } from '../components/services/ServicesPanel';
import { useServiceStore } from '../stores/useServiceStore';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import { Card } from '../components/ui/Card';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/button';
import type { WorkspaceSummary } from '../types';

interface ServicesHeaderProps {
  subtitle: string;
  isCloud: boolean;
  workspaces: WorkspaceSummary[];
  workspaceFilter: string | null;
  onWorkspaceChange: (workspaceId: string | null) => void;
}

function ServicesHeader({
  subtitle,
  isCloud,
  workspaces,
  workspaceFilter,
  onWorkspaceChange,
}: ServicesHeaderProps) {
  return (
    <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
      <div>
        <h1 className="text-3xl font-semibold text-foreground">Services</h1>
        <p className="text-base text-muted-foreground mt-1">{subtitle}</p>
      </div>
      {isCloud && (
        <div className="flex items-center gap-2">
          <label htmlFor="services-workspace-filter" className="text-sm text-muted-foreground">
            Workspace
          </label>
          <select
            id="services-workspace-filter"
            value={workspaceFilter ?? ''}
            onChange={(e) => {
              const value = e.target.value;
              onWorkspaceChange(value === '' ? null : value);
            }}
            className="h-9 rounded-md border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <option value="">All workspaces</option>
            {workspaces.map((w) => (
              <option key={w.id} value={w.id}>{w.name}</option>
            ))}
          </select>
        </div>
      )}
    </div>
  );
}

export function ServicesPage() {
  const {
    services,
    updateService,
    toggleRoute,
    filteredRoutes,
    isLoading,
    error,
    fetchServices,
    clearError,
    isCloud,
    workspaceFilter,
    setWorkspaceFilter,
  } = useServiceStore();
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const loadWorkspaces = useWorkspaceStore((s) => s.loadWorkspaces);

  useEffect(() => {
    fetchServices();
  }, [fetchServices]);

  useEffect(() => {
    if (isCloud && workspaces.length === 0) {
      void loadWorkspaces();
    }
  }, [isCloud, workspaces.length, loadWorkspaces]);

  const handleWorkspaceChange = (workspaceId: string | null) => {
    void setWorkspaceFilter(workspaceId);
  };

  const totalRoutes = services.reduce((acc, s) => acc + s.routes.length, 0);
  const searchActive = filteredRoutes.length !== totalRoutes;
  const hasServices = services.length > 0;

  const header = (subtitle: string) => (
    <ServicesHeader
      subtitle={subtitle}
      isCloud={isCloud}
      workspaces={workspaces}
      workspaceFilter={workspaceFilter}
      onWorkspaceChange={handleWorkspaceChange}
    />
  );

  if (isLoading) {
    return (
      <div className="space-y-8">
        {header('Loading services...')}
        <Card title="Loading" icon={<RefreshCw className="h-4 w-4 animate-spin" />}>
          <div className="text-sm text-muted-foreground">Fetching service configuration...</div>
        </Card>
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-8">
        {header('Manage services and routes.')}
        <Card title="Error Loading Services" icon={<AlertCircle className="h-4 w-4 text-danger-500" />}>
          <div className="space-y-4">
            <div className="text-sm text-danger-600 dark:text-danger-400">{error}</div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                clearError();
                fetchServices();
              }}
              className="flex items-center gap-2"
            >
              <RefreshCw className="h-4 w-4" />
              Retry
            </Button>
          </div>
        </Card>
      </div>
    );
  }

  if (!hasServices && !isCloud) {
    // In self-hosted mode there's no "create service" control, so render
    // a static empty-state card.
    return (
      <div className="space-y-8">
        {header('Manage services and routes. Use global search to quickly filter routes.')}
        <Card title="No Services" icon={<Search className="h-4 w-4" />}>
          <div className="text-sm text-muted-foreground">
            No services configured. Add a service to get started.
          </div>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {header('Manage services and routes. Use global search to quickly filter routes.')}

      <Card title="Matching Routes" icon={<Search className="h-4 w-4" />} className={searchActive ? '' : 'opacity-70'}>
        {searchActive ? (
          <div className="space-y-3">
            <div className="text-sm text-muted-foreground">{filteredRoutes.length} routes match your search.</div>
            <ul className="divide-y divide-border rounded-md border border-border bg-bg-primary">
              {filteredRoutes.slice(0, 10).map((r) => (
                <li key={r.id} className="px-4 py-3 flex items-center gap-3">
                  <Badge variant="brand" className="uppercase">{r.method || 'ANY'}</Badge>
                  <span className="font-mono text-sm text-foreground truncate" title={r.path}>{r.path}</span>
                </li>
              ))}
            </ul>
            {filteredRoutes.length > 10 && (
              <div className="text-xs text-muted-foreground">Showing first 10 results… refine your query to narrow further.</div>
            )}
          </div>
        ) : (
          <div className="text-sm text-muted-foreground">No search active. Type in the header's global search to filter routes.</div>
        )}
      </Card>

      <ServicesPanel
        services={services}
        onUpdateService={updateService}
        onToggleRoute={toggleRoute}
      />
    </div>
  );
}
