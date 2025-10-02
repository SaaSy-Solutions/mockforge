import React, { useState } from 'react';
import { Switch } from '../ui/switch';
import { Button } from '../ui/button';
import { ContextMenuWithItems, type ContextMenuItem } from '../ui/ContextMenu';
import type { ServiceInfo } from '../../types';
import { cn } from '../../utils/cn';
import { generateCurlCommand, copyToClipboard } from '../../utils/curlGenerator';

interface ServiceToggleCardProps {
  service: ServiceInfo;
  onToggleService: (serviceId: string, enabled: boolean) => void;
  onToggleRoute: (serviceId: string, routeId: string, enabled: boolean) => void;
  expanded?: boolean;
  onToggleExpanded: (serviceId: string) => void;
}

export function ServiceToggleCard({
  service,
  onToggleService,
  onToggleRoute,
  expanded = false,
  onToggleExpanded
}: ServiceToggleCardProps) {
  const enabledRoutes = service.routes.filter(route => route.enabled !== false).length;
  const totalRoutes = service.routes.length;

  // Context menu state
  const [contextMenu, setContextMenu] = useState<{
    visible: boolean;
    position: { x: number; y: number };
    route: ServiceInfo['routes'][0] | null;
  }>({
    visible: false,
    position: { x: 0, y: 0 },
    route: null
  });

  const handleRouteRightClick = (event: React.MouseEvent, route: ServiceInfo['routes'][0]) => {
    event.preventDefault();
    setContextMenu({
      visible: true,
      position: { x: event.clientX, y: event.clientY },
      route
    });
  };

  const handleContextMenuClose = () => {
    setContextMenu(prev => ({ ...prev, visible: false }));
  };

  const handleCopyCurl = async (route: ServiceInfo['routes'][0]) => {
    const curlCommand = generateCurlCommand(route);
    const success = await copyToClipboard(curlCommand);

    if (success) {
      // Could add a toast notification here in the future
      console.log('cURL command copied to clipboard:', curlCommand);
    } else {
      console.error('Failed to copy cURL command to clipboard');
    }
  };

  const getContextMenuItems = (route: ServiceInfo['routes'][0]): ContextMenuItem[] => [
    {
      label: 'Copy as cURL',
      onClick: () => handleCopyCurl(route),
      icon: '📋'
    }
  ];

  return (
    <div className="rounded-lg border bg-card">
      <div className="p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <Switch
              checked={service.enabled}
              onCheckedChange={(checked) => onToggleService(service.id, checked)}
            />
            <div>
              <h3 className="font-semibold">{service.name}</h3>
              <p className="text-sm text-muted-foreground">
                {enabledRoutes}/{totalRoutes} routes enabled
              </p>
            </div>
          </div>
          <div className="flex items-center space-x-2">
            {service.tags?.map(tag => (
              <span
                key={tag}
                className="inline-flex items-center rounded-full bg-secondary px-2 py-1 text-xs font-medium"
              >
                {tag}
              </span>
            ))}
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onToggleExpanded(service.id)}
            >
              {expanded ? '↑' : '↓'}
            </Button>
          </div>
        </div>

        {service.description && (
          <p className="mt-2 text-sm text-muted-foreground">
            {service.description}
          </p>
        )}
      </div>

      {expanded && (
        <div className="border-t p-4 space-y-2">
          <h4 className="font-medium text-sm mb-3">Routes</h4>
          {service.routes.map((route) => (
            <div
              key={`${route.method}-${route.path}`}
              className="flex items-center justify-between py-2 px-3 rounded border bg-background cursor-context-menu"
              onContextMenu={(event) => handleRouteRightClick(event, route)}
            >
              <div className="flex items-center space-x-3">
                <Switch
                  checked={route.enabled !== false}
                  onCheckedChange={(checked) =>
                    onToggleRoute(service.id, `${route.method}-${route.path}`, checked)
                  }
                  disabled={!service.enabled}
                />
                <div className="flex items-center space-x-2">
                  <span className={cn(
                    "text-xs font-mono px-2 py-1 rounded",
                    {
                      'bg-green-100 text-green-800': route.method === 'GET',
                      'bg-blue-100 text-blue-800': route.method === 'POST',
                      'bg-yellow-100 text-yellow-800': route.method === 'PUT',
                      'bg-red-100 text-red-800': route.method === 'DELETE',
                      'bg-purple-100 text-purple-800': route.method === 'PATCH',
                      'bg-gray-100 text-gray-800': !route.method,
                    }
                  )}>
                    {route.method || 'gRPC'}
                  </span>
                  <span className="font-mono text-sm">{route.path}</span>
                </div>
              </div>

              <div className="flex items-center space-x-4 text-xs text-muted-foreground">
                <span>{route.request_count} requests</span>
                {route.latency_ms && (
                  <span>{route.latency_ms}ms avg</span>
                )}
                {(route.error_count || 0) > 0 && (
                  <span className="text-destructive">{route.error_count || 0} errors</span>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Context Menu */}
      {contextMenu.visible && contextMenu.route && (
        <ContextMenuWithItems
          items={getContextMenuItems(contextMenu.route)}
          position={contextMenu.position}
          onClose={handleContextMenuClose}
        />
      )}
    </div>
  );
}
