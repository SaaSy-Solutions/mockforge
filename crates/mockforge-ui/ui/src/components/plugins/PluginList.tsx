import React, { useState, useEffect, useCallback } from 'react';
import {
  MoreHorizontal,
  Eye,
  Trash2,
  RefreshCw,
  CheckCircle,
  XCircle,
  AlertTriangle,
  Settings
} from 'lucide-react';
import {
  Card,
  Badge,
  Button,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  Alert
} from '../ui/DesignSystem';

interface Plugin {
  id: string;
  name: string;
  version: string;
  types: string[];
  status: string;
  healthy: boolean;
  description: string;
  author: string;
}

interface PluginListProps {
  filterType: string;
  filterStatus: string;
  onSelectPlugin: (pluginId: string) => void;
}

export function PluginList({ filterType, filterStatus, onSelectPlugin }: PluginListProps) {
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchPlugins = useCallback(async () => {
    try {
      setLoading(true);
      const params = new URLSearchParams();
      if (filterType) params.append('type', filterType);
      if (filterStatus) params.append('status', filterStatus);

      const response = await fetch(`/__mockforge/plugins?${params}`);
      const data = await response.json();

      if (data.success) {
        setPlugins(data.data.plugins);
      } else {
        setError(data.error);
      }
    } catch {
      setError('Failed to fetch plugins');
    } finally {
      setLoading(false);
    }
  }, [filterType, filterStatus]);

  useEffect(() => {
    fetchPlugins();
  }, [fetchPlugins]);

  const handleDeletePlugin = async (pluginId: string) => {
    if (!confirm(`Are you sure you want to delete plugin "${pluginId}"?`)) {
      return;
    }

    try {
      const response = await fetch(`/__mockforge/plugins/${pluginId}`, {
        method: 'DELETE',
      });
      const data = await response.json();

      if (data.success) {
        fetchPlugins(); // Refresh the list
      } else {
        alert(`Failed to delete plugin: ${data.error}`);
      }
    } catch {
      alert('Failed to delete plugin');
    }
  };

  const handleReloadPlugin = async (pluginId: string) => {
    try {
      const response = await fetch('/__mockforge/plugins/reload', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ plugin_id: pluginId }),
      });
      const data = await response.json();

      if (data.success) {
        fetchPlugins(); // Refresh the list
      } else {
        alert(`Failed to reload plugin: ${data.error}`);
      }
    } catch {
      alert('Failed to reload plugin');
    }
  };

  const getStatusBadge = (status: string, healthy: boolean) => {
    const variant = healthy ? 'success' : 'destructive';
    const icon = healthy ? <CheckCircle className="w-3 h-3" /> : <XCircle className="w-3 h-3" />;

    return (
      <Badge variant={variant} className="flex items-center gap-1">
        {icon}
        {status}
      </Badge>
    );
  };

  const getTypeBadges = (types: string[]) => {
    return types.map(type => (
      <Badge key={type} variant="outline" className="text-xs">
        {type}
      </Badge>
    ));
  };

  if (loading) {
    return (
      <Card>
        <div className="p-6">
          <div className="animate-pulse space-y-4">
            <div className="h-4 bg-gray-200 rounded w-1/4"></div>
            <div className="h-4 bg-gray-200 rounded w-1/2"></div>
            <div className="h-4 bg-gray-200 rounded w-3/4"></div>
          </div>
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertTriangle className="h-4 w-4" />
        <div>
          <strong>Error loading plugins:</strong> {error}
        </div>
      </Alert>
    );
  }

  if (plugins.length === 0) {
    return (
      <Card>
        <div className="p-6 text-center">
          <div className="text-gray-500">
            <Settings className="w-12 h-12 mx-auto mb-4" />
            <h3 className="text-lg font-medium mb-2">No plugins installed</h3>
            <p className="mb-4">Install plugins to extend MockForge functionality</p>
            <Button variant="outline">Browse Marketplace</Button>
          </div>
        </div>
      </Card>
    );
  }

  return (
    <Card>
      <div className="p-6">
        <div className="flex justify-between items-center mb-4">
          <h3 className="text-lg font-medium">Installed Plugins ({plugins.length})</h3>
          <Button variant="outline" size="sm" onClick={fetchPlugins}>
            <RefreshCw className="w-4 h-4" />
          </Button>
        </div>

        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
            <thead className="bg-gray-50 dark:bg-gray-800">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Name</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Version</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Types</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Status</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Author</th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider w-12"></th>
              </tr>
            </thead>
            <tbody className="bg-white dark:bg-gray-900 divide-y divide-gray-200 dark:divide-gray-700">
              {plugins.map((plugin) => (
                <tr key={plugin.id} className="hover:bg-gray-50 dark:hover:bg-gray-800">
                  <td className="px-6 py-4">
                    <div>
                      <div className="font-medium text-gray-900 dark:text-gray-100">{plugin.name}</div>
                      <div className="text-sm text-gray-500 dark:text-gray-400">{plugin.description}</div>
                    </div>
                  </td>
                  <td className="px-6 py-4 text-sm text-gray-900 dark:text-gray-100">{plugin.version}</td>
                  <td className="px-6 py-4">
                    <div className="flex gap-1 flex-wrap">
                      {getTypeBadges(plugin.types)}
                    </div>
                  </td>
                  <td className="px-6 py-4">{getStatusBadge(plugin.status, plugin.healthy)}</td>
                  <td className="px-6 py-4 text-sm text-gray-900 dark:text-gray-100">{plugin.author}</td>
                  <td className="px-6 py-4">
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="sm">
                          <MoreHorizontal className="w-4 h-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => onSelectPlugin(plugin.id)}>
                          <Eye className="w-4 h-4 mr-2" />
                          View Details
                        </DropdownMenuItem>
                        <DropdownMenuItem onClick={() => handleReloadPlugin(plugin.id)}>
                          <RefreshCw className="w-4 h-4 mr-2" />
                          Reload
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={() => handleDeletePlugin(plugin.id)}
                          className="text-red-600"
                        >
                          <Trash2 className="w-4 h-4 mr-2" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </Card>
  );
}
