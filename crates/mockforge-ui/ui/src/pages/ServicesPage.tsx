import React from 'react';
import { ServicesPanel } from '../components/services/ServicesPanel';
import { useServiceStore } from '../stores/useServiceStore';
import { Card } from '../components/ui/Card';
import { Badge } from '../components/ui/Badge';

export function ServicesPage() {
  const { services, updateService, toggleRoute, filteredRoutes } = useServiceStore();

  const totalRoutes = services.reduce((acc, s) => acc + s.routes.length, 0);
  const searchActive = filteredRoutes.length !== totalRoutes;

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-semibold text-gray-900 dark:text-gray-100">Services</h1>
        <p className="text-base text-gray-600 dark:text-gray-400 mt-1">Manage services and routes. Use global search to quickly filter routes.</p>
      </div>

      <Card title="Matching Routes" icon={<span className="text-sm font-bold">🔎</span>} className={searchActive ? '' : 'opacity-70'}>
        {searchActive ? (
          <div className="space-y-3">
            <div className="text-sm text-gray-600 dark:text-gray-400">{filteredRoutes.length} routes match your search.</div>
            <ul className="divide-y divide-border rounded-md border border-border bg-bg-primary">
              {filteredRoutes.slice(0, 10).map((r, idx) => (
                <li key={`${r.method ?? 'ANY'}-${r.path}-${idx}`} className="px-4 py-3 flex items-center gap-3">
                  {r.method && <Badge variant="brand" className="uppercase">{r.method}</Badge>}
                  <span className="font-mono text-sm text-gray-900 dark:text-gray-100 truncate" title={r.path}>{r.path}</span>
                </li>
              ))}
            </ul>
            {filteredRoutes.length > 10 && (
              <div className="text-xs text-gray-600 dark:text-gray-400">Showing first 10 results… refine your query to narrow further.</div>
            )}
          </div>
        ) : (
          <div className="text-sm text-gray-600 dark:text-gray-400">No search active. Type in the header’s global search to filter routes.</div>
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


