import React, { useState, useEffect } from 'react';
import {
  MoreHorizontal,
  Eye,
  Trash2,
  RefreshCw,
  CheckCircle,
  XCircle,
  AlertTriangle,
  Settings,
  Loader2
} from 'lucide-react';
import {
  Card,
  Table,
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

  useEffect(() => {
    fetchPlugins();
  }, []);

  const fetchPlugins = async () => {
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
    } catch (err) {
      setError('Failed to fetch plugins');
    } finally {
      setLoading(false);
    }
  };

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
    } catch (err) {
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
    } catch (err) {
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

        <Table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Version</th>
              <th>Types</th>
              <th>Status</th>
              <th>Author</th>
              <th className="w-12"></th>
            </tr>
          </thead>
          <tbody>
            {plugins.map((plugin) => (
              <tr key={plugin.id} className="hover:bg-gray-50">
                <td>
                  <div>
                    <div className="font-medium">{plugin.name}</div>
                    <div className="text-sm text-gray-500">{plugin.description}</div>
                  </div>
                </td>
                <td className="text-sm">{plugin.version}</td>
                <td>
                  <div className="flex gap-1 flex-wrap">
                    {getTypeBadges(plugin.types)}
                  </div>
                </td>
                <td>{getStatusBadge(plugin.status, plugin.healthy)}</td>
                <td className="text-sm">{plugin.author}</td>
                <td>
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
        </Table>
      </div>
    </Card>
  );
}
