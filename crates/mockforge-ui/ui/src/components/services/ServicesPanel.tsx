import React, { useMemo, useState } from 'react';
import { ServiceToggleCard } from './ServiceToggleCard';
import { Button } from '../ui/button';
import type { ServiceInfo } from '../../types';
import { useServiceStore } from '../../stores/useServiceStore';
import { Badge } from '../ui/Badge';

interface ServicesPanelProps {
  services: ServiceInfo[];
  onUpdateService: (serviceId: string, updates: Partial<ServiceInfo>) => void;
  onToggleRoute: (serviceId: string, routeId: string, enabled: boolean) => void;
}

export function ServicesPanel({ services, onUpdateService, onToggleRoute }: ServicesPanelProps) {
  const [expandedServices, setExpandedServices] = useState<Set<string>>(new Set());
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const { filteredRoutes } = useServiceStore();
  const [showAllMatches, setShowAllMatches] = useState(false);
  const [page, setPage] = useState(1);
  const pageSize = 20;

  // Get all unique tags from services
  const allTags = Array.from(
    new Set(services.flatMap(service => service.tags || []))
  );

  // Filter services based on search and tags
  const filteredServices = services.filter(service => {
    const matchesSearch = searchTerm === '' ||
      service.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      service.routes.some(route => route.path.toLowerCase().includes(searchTerm.toLowerCase()));

    const matchesTags = selectedTags.size === 0 ||
      (service.tags && service.tags.some(tag => selectedTags.has(tag)));

    return matchesSearch && matchesTags;
  });

  const handleToggleService = (serviceId: string, enabled: boolean) => {
    onUpdateService(serviceId, { enabled });
  };

  const handleToggleExpanded = (serviceId: string) => {
    const newExpanded = new Set(expandedServices);
    if (newExpanded.has(serviceId)) {
      newExpanded.delete(serviceId);
    } else {
      newExpanded.add(serviceId);
    }
    setExpandedServices(newExpanded);
  };

  const handleBulkEnable = () => {
    filteredServices.forEach(service => {
      onUpdateService(service.id, { enabled: true });
    });
  };

  const handleBulkDisable = () => {
    filteredServices.forEach(service => {
      onUpdateService(service.id, { enabled: false });
    });
  };

  const handleTagToggle = (tag: string) => {
    const newTags = new Set(selectedTags);
    if (newTags.has(tag)) {
      newTags.delete(tag);
    } else {
      newTags.add(tag);
    }
    setSelectedTags(newTags);
  };

  const enabledServices = filteredServices.filter(s => s.enabled).length;
  const totalRoutes = useMemo(() => services.reduce((acc, s) => acc + s.routes.length, 0), [services]);
  const searchActive = filteredRoutes.length !== totalRoutes;
  const previewRoutes = filteredRoutes.slice(0, 3);
  const totalPages = Math.max(1, Math.ceil(filteredRoutes.length / pageSize));
  const paged = filteredRoutes.slice((page - 1) * pageSize, page * pageSize);

  return (
    <div className="space-y-6">
      {/* Header Controls */}
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <h2 className="text-2xl font-bold">Services</h2>
          <p className="text-muted-foreground">
            {enabledServices}/{filteredServices.length} services enabled
          </p>
        </div>
        {searchActive && (
          <div className="flex-1 md:flex-none">
            <div className="flex items-center gap-3">
              <span className="text-sm text-muted-foreground">Matching Routes: {filteredRoutes.length}</span>
              <div className="hidden lg:flex items-center gap-2">
                {previewRoutes.map((r, idx) => (
                  <div key={`${r.method ?? 'ANY'}-${r.path}-${idx}`} className="inline-flex items-center gap-2 px-2.5 py-1 rounded-md border border-border bg-bg-primary">
                    {r.method && <Badge variant="brand" className="uppercase">{r.method}</Badge>}
                    <span className="font-mono text-xs text-text-secondary max-w-[240px] truncate" title={r.path}>{r.path}</span>
                  </div>
                ))}
              </div>
              <Button variant="outline" size="sm" onClick={() => setShowAllMatches(true)}>View all</Button>
            </div>
          </div>
        )}
        <div className="flex items-center space-x-2">
          <Button variant="outline" size="sm" onClick={handleBulkEnable}>
            Enable All
          </Button>
          <Button variant="outline" size="sm" onClick={handleBulkDisable}>
            Disable All
          </Button>
        </div>
      </div>

      {/* Search and Filters */}
      <div className="space-y-4">
        <input
          type="text"
          placeholder="Search services and routes..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="w-full px-3 py-2 border border-input rounded-md bg-background text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />

        {allTags.length > 0 && (
          <div className="space-y-2">
            <h3 className="text-sm font-medium">Filter by tags:</h3>
            <div className="flex flex-wrap gap-2">
              {allTags.map(tag => (
                <button
                  key={tag}
                  onClick={() => handleTagToggle(tag)}
                  className={`inline-flex items-center rounded-full px-3 py-1 text-xs font-medium transition-colors ${
                    selectedTags.has(tag)
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-secondary text-secondary-foreground hover:bg-secondary/80'
                  }`}
                >
                  {tag}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Services List */}
      <div className="space-y-4">
        {filteredServices.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            No services found matching your criteria.
          </div>
        ) : (
          filteredServices.map(service => (
            <ServiceToggleCard
              key={service.id}
              service={service}
              onToggleService={handleToggleService}
              onToggleRoute={onToggleRoute}
              expanded={expandedServices.has(service.id)}
              onToggleExpanded={handleToggleExpanded}
            />
          ))
        )}
      </div>

      {/* View all matches Modal */}
      {showAllMatches && (
        <div className="fixed inset-0 z-50">
          <div className="fixed inset-0 bg-bg-overlay" onClick={() => setShowAllMatches(false)} />
          <div className="fixed inset-x-0 top-16 mx-auto w-full max-w-3xl bg-bg-primary border border-border rounded-xl shadow-xl">
            <div className="flex items-center justify-between px-6 py-4 border-b border-border">
              <h3 className="text-lg font-semibold text-text-primary">Matching Routes ({filteredRoutes.length})</h3>
              <div className="flex items-center gap-2">
                <span className="text-xs text-text-tertiary">Page {page} / {totalPages}</span>
                <Button size="sm" variant="ghost" onClick={() => setShowAllMatches(false)}>Close</Button>
              </div>
            </div>
            <div className="max-h-[60vh] overflow-y-auto">
              <ul className="divide-y divide-border">
                {paged.map((r, idx) => (
                  <li key={`${r.method ?? 'ANY'}-${r.path}-${idx}`} className="px-6 py-3 flex items-center gap-3">
                    {r.method && <Badge variant="brand" className="uppercase">{r.method}</Badge>}
                    <span className="font-mono text-sm text-text-primary truncate" title={r.path}>{r.path}</span>
                    {r.tags && r.tags.length > 0 && (
                      <span className="ml-auto text-xs text-text-tertiary">{r.tags.join(', ')}</span>
                    )}
                  </li>
                ))}
              </ul>
            </div>
            <div className="flex items-center justify-between px-6 py-3 border-t border-border bg-bg-secondary/30">
              <Button size="sm" variant="outline" onClick={() => setPage(p => Math.max(1, p - 1))} disabled={page === 1}>Previous</Button>
              <div className="text-xs text-text-tertiary">Showing {(page - 1) * pageSize + 1} - {Math.min(page * pageSize, filteredRoutes.length)} of {filteredRoutes.length}</div>
              <Button size="sm" variant="outline" onClick={() => setPage(p => Math.min(totalPages, p + 1))} disabled={page === totalPages}>Next</Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
