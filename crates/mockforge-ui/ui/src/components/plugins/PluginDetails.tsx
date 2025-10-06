import { logger } from '@/utils/logger';
import React, { useState, useEffect, useCallback } from 'react';
import {
  X,
  CheckCircle,
  XCircle,
  AlertTriangle,
  User,
  Globe,
  Shield,
  HardDrive,
  Network,
  Clock,
  RefreshCw,
  Trash2
} from 'lucide-react';
import {
  Modal,
  Card,
  Badge,
  Button,
  Alert
} from '../ui/DesignSystem';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '../ui/Tabs';

interface NetworkCapabilities {
  allow_http_outbound: boolean;
  allowed_hosts: string[];
}

interface FilesystemCapabilities {
  allow_read: boolean;
  allow_write: boolean;
  allowed_paths: string[];
}

interface ResourceCapabilities {
  max_memory_bytes: number;
  max_cpu_percent: number;
}

interface PluginDetails {
  id: string;
  name: string;
  version: string;
  types: string[];
  status: string;
  healthy: boolean;
  description: string;
  author: string;
  homepage?: string;
  repository?: string;
  capabilities: {
    network: NetworkCapabilities;
    filesystem: FilesystemCapabilities;
    resources: ResourceCapabilities;
  };
  health: {
    status: string;
    message: string;
    last_check: string;
  };
}

interface PluginDetailsProps {
  pluginId: string;
  onClose: () => void;
}

export function PluginDetails({ pluginId, onClose }: PluginDetailsProps) {
  const [plugin, setPlugin] = useState<PluginDetails | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchPluginDetails = useCallback(async () => {
    try {
      setLoading(true);
      const response = await fetch(`/__mockforge/plugins/${pluginId}`);
      const data = await response.json();

      if (data.success) {
        setPlugin(data.data);
      } else {
        setError(data.error);
      }
    } catch {
      setError('Failed to fetch plugin details');
    } finally {
      setLoading(false);
    }
  }, [pluginId]);

  useEffect(() => {
    fetchPluginDetails();
  }, [fetchPluginDetails]);

  const getStatusIcon = (healthy: boolean) => {
    return healthy ? (
      <CheckCircle className="w-5 h-5 text-green-500" />
    ) : (
      <XCircle className="w-5 h-5 text-red-500" />
    );
  };

  const getTypeBadges = (types: string[]) => {
    return types.map(type => (
      <Badge key={type} variant="outline" className="text-xs">
        {type}
      </Badge>
    ));
  };

  const formatDateTime = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

  if (loading) {
    return (
      <Modal open={true} onOpenChange={onClose}>
        <div className="p-6">
          <div className="animate-pulse space-y-4">
            <div className="h-8 bg-gray-200 rounded w-1/3"></div>
            <div className="h-4 bg-gray-200 rounded w-1/2"></div>
            <div className="h-32 bg-gray-200 rounded"></div>
          </div>
        </div>
      </Modal>
    );
  }

  if (error || !plugin) {
    return (
      <Modal open={true} onOpenChange={onClose}>
        <div className="p-6">
          <Alert variant="destructive">
            <AlertTriangle className="h-4 w-4" />
            <div>
              <strong>Error:</strong> {error || 'Plugin not found'}
            </div>
          </Alert>
          <div className="mt-4 flex justify-end">
            <Button onClick={onClose}>Close</Button>
          </div>
        </div>
      </Modal>
    );
  }

  return (
    <Modal open={true} onOpenChange={onClose} className="max-w-4xl">
      <div className="p-6">
        <div className="flex justify-between items-start mb-6">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              {getStatusIcon(plugin.healthy)}
              <h2 className="text-2xl font-bold">{plugin.name}</h2>
            </div>
            <Badge variant="outline">{plugin.version}</Badge>
          </div>
          <Button variant="ghost" size="sm" onClick={onClose}>
            <X className="w-4 h-4" />
          </Button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
          <div className="space-y-4">
            <div>
              <h3 className="font-semibold mb-2">Basic Information</h3>
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <span className="text-sm text-gray-600">ID:</span>
                  <code className="text-sm bg-gray-100 px-2 py-1 rounded">{plugin.id}</code>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-sm text-gray-600">Types:</span>
                  <div className="flex gap-1">
                    {getTypeBadges(plugin.types)}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-sm text-gray-600">Status:</span>
                  <Badge variant={plugin.healthy ? 'success' : 'destructive'}>
                    {plugin.status}
                  </Badge>
                </div>
                <div className="flex items-center gap-2">
                  <User className="w-4 h-4 text-gray-600" />
                  <span className="text-sm">{plugin.author}</span>
                </div>
                {plugin.homepage && (
                  <div className="flex items-center gap-2">
                    <Globe className="w-4 h-4 text-gray-600" />
                    <a
                      href={plugin.homepage}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-sm text-blue-600 hover:underline"
                    >
                      {plugin.homepage}
                    </a>
                  </div>
                )}
              </div>
            </div>

            <div>
              <h3 className="font-semibold mb-2">Description</h3>
              <p className="text-sm text-gray-600">{plugin.description}</p>
            </div>
          </div>

          <div className="space-y-4">
            <div>
              <h3 className="font-semibold mb-2">Health Status</h3>
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <span className="text-sm text-gray-600">Status:</span>
                  <Badge variant={plugin.health.status === 'healthy' ? 'success' : 'destructive'}>
                    {plugin.health.status}
                  </Badge>
                </div>
                <div className="flex items-center gap-2">
                  <Clock className="w-4 h-4 text-gray-600" />
                  <span className="text-sm">
                    Last checked: {formatDateTime(plugin.health.last_check)}
                  </span>
                </div>
                {plugin.health.message && (
                  <Alert variant={plugin.healthy ? 'default' : 'destructive'} className="text-sm">
                    {plugin.health.message}
                  </Alert>
                )}
              </div>
            </div>
          </div>
        </div>

        <Tabs defaultValue="capabilities" className="w-full">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="capabilities">Capabilities</TabsTrigger>
            <TabsTrigger value="resources">Resources</TabsTrigger>
            <TabsTrigger value="actions">Actions</TabsTrigger>
          </TabsList>

          <TabsContent value="capabilities" className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <Card>
                <div className="p-4">
                  <div className="flex items-center gap-2 mb-3">
                    <Network className="w-4 h-4" />
                    <h4 className="font-medium">Network Access</h4>
                  </div>
                  <div className="space-y-1 text-sm">
                    <div>Outbound HTTP: {plugin.capabilities.network.allow_http_outbound ? 'Allowed' : 'Blocked'}</div>
                    <div>Allowed Hosts: {plugin.capabilities.network.allowed_hosts.join(', ') || 'None'}</div>
                  </div>
                </div>
              </Card>

              <Card>
                <div className="p-4">
                  <div className="flex items-center gap-2 mb-3">
                    <HardDrive className="w-4 h-4" />
                    <h4 className="font-medium">File System</h4>
                  </div>
                  <div className="space-y-1 text-sm">
                    <div>Read Access: {plugin.capabilities.filesystem.allow_read ? 'Allowed' : 'Blocked'}</div>
                    <div>Write Access: {plugin.capabilities.filesystem.allow_write ? 'Allowed' : 'Blocked'}</div>
                    <div>Allowed Paths: {plugin.capabilities.filesystem.allowed_paths.join(', ') || 'None'}</div>
                  </div>
                </div>
              </Card>
            </div>
          </TabsContent>

          <TabsContent value="resources" className="space-y-4">
            <Card>
              <div className="p-4">
                <div className="flex items-center gap-2 mb-3">
                  <Shield className="w-4 h-4" />
                  <h4 className="font-medium">Resource Limits</h4>
                </div>
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <div className="text-gray-600">Max Memory</div>
                    <div className="font-medium">
                      {(plugin.capabilities.resources.max_memory_bytes / 1024 / 1024).toFixed(0)} MB
                    </div>
                  </div>
                  <div>
                    <div className="text-gray-600">Max CPU</div>
                    <div className="font-medium">
                      {plugin.capabilities.resources.max_cpu_percent}%
                    </div>
                  </div>
                </div>
              </div>
            </Card>
          </TabsContent>

          <TabsContent value="actions" className="space-y-4">
            <div className="flex gap-3">
              <Button variant="outline">
                <RefreshCw className="w-4 h-4 mr-2" />
                Reload Plugin
              </Button>
              <Button variant="outline">
                Enable/Disable
              </Button>
              <Button variant="destructive">
                <Trash2 className="w-4 h-4 mr-2" />
                Uninstall
              </Button>
            </div>
          </TabsContent>
        </Tabs>
      </div>
    </Modal>
  );
}
