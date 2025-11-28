import { logger } from '@/utils/logger';
import React, { useState, useCallback } from 'react';
import {
  Puzzle,
  Plus,
  RefreshCw,
  Upload,
  Search,
  AlertCircle,
} from 'lucide-react';
import {
  PageHeader,
  Button,
  Card,
  Input,
  EmptyState,
  Alert,
} from '../components/ui/DesignSystem';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '../components/ui/Tabs';
import { PluginList } from '../components/plugins/PluginList';
import { PluginDetails } from '../components/plugins/PluginDetails';
import { PluginStatus } from '../components/plugins/PluginStatus';
import { InstallPluginModal } from '../components/plugins/InstallPluginModal';
import type { PluginType, PluginStatus as PluginStatusType } from '../types';
import { pluginsApi } from '../services/api';

export function PluginsPage() {
  const [activeTab, setActiveTab] = useState('installed');
  const [selectedPlugin, setSelectedPlugin] = useState<string | null>(null);
  const [showInstallModal, setShowInstallModal] = useState(false);
  const [filterType, setFilterType] = useState<PluginType | ''>('');
  const [filterStatus, setFilterStatus] = useState<PluginStatusType | ''>('');
  const [searchQuery, setSearchQuery] = useState('');
  const [isLoading, _setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isReloading, setIsReloading] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);

  const handleReloadAll = useCallback(async () => {
    setIsReloading(true);
    setError(null);
    try {
      const result = await pluginsApi.reloadAllPlugins();

      // Trigger a refresh of the plugin list
      setReloadKey(prev => prev + 1);

      // Optionally show success message (you could add a toast/notification system)
      logger.info(result.message);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to reload plugins');
    } finally {
      setIsReloading(false);
    }
  }, []);

  const handleBrowseMarketplace = useCallback(() => {
    // Open marketplace page (if it exists) or navigate to marketplace section
    const marketplacePath = '/plugins/marketplace';
    // Try to navigate within the app first
    if (window.location.hash) {
      window.location.hash = 'plugins-marketplace';
    } else {
      // Fallback to opening in new tab
      window.open(marketplacePath, '_blank');
    }
  }, []);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Plugin Management"
        subtitle="Manage authentication, template, response, and datasource plugins"
        action={
          <div className="flex gap-3">
            <Button
              variant="outline"
              onClick={() => setShowInstallModal(true)}
              className="flex items-center gap-2"
              disabled={isLoading}
              aria-label="Install new plugin"
            >
              <Plus className="w-4 h-4" />
              Install Plugin
            </Button>
            <Button
              variant="outline"
              className="flex items-center gap-2"
              onClick={handleReloadAll}
              disabled={isReloading}
              aria-label="Reload all plugins"
            >
              <RefreshCw className={`w-4 h-4 ${isReloading ? 'animate-spin' : ''}`} />
              Reload All
            </Button>
          </div>
        }
      />

      {error && (
        <Alert type="error" title="Error">
          <div className="flex items-center gap-2">
            <AlertCircle className="w-4 h-4" />
            <span>{error}</span>
          </div>
        </Alert>
      )}

      <div className="flex gap-4">
        <div className="flex-1 relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-gray-400" />
          <Input
            placeholder="Search plugins by name or description..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-10"
            aria-label="Search plugins"
          />
        </div>
        <div className="w-64">
          <Input
            placeholder="Filter by type"
            value={filterType}
            onChange={(e) => setFilterType(e.target.value as PluginType | '')}
            aria-label="Filter plugins by type"
            list="plugin-types"
          />
          <datalist id="plugin-types">
            <option value="authentication" />
            <option value="template" />
            <option value="response" />
            <option value="datasource" />
          </datalist>
        </div>
        <div className="w-64">
          <Input
            placeholder="Filter by status"
            value={filterStatus}
            onChange={(e) => setFilterStatus(e.target.value as PluginStatusType | '')}
            aria-label="Filter plugins by status"
            list="plugin-statuses"
          />
          <datalist id="plugin-statuses">
            <option value="active" />
            <option value="inactive" />
            <option value="error" />
            <option value="loading" />
          </datalist>
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList>
          <TabsTrigger value="installed">Installed Plugins</TabsTrigger>
          <TabsTrigger value="status">System Status</TabsTrigger>
          <TabsTrigger value="marketplace">Marketplace</TabsTrigger>
        </TabsList>

        <TabsContent value="installed" className="space-y-6">
          {isLoading ? (
            <Card>
              <div className="p-8 text-center">
                <RefreshCw className="w-8 h-8 animate-spin mx-auto mb-4 text-gray-400" />
                <p className="text-gray-600">Loading plugins...</p>
              </div>
            </Card>
          ) : (
            <PluginList
              key={reloadKey}
              filterType={filterType}
              filterStatus={filterStatus}
              onSelectPlugin={setSelectedPlugin}
            />
          )}
        </TabsContent>

        <TabsContent value="status" className="space-y-6">
          <PluginStatus />
        </TabsContent>

        <TabsContent value="marketplace" className="space-y-6">
          <Card>
            <div className="p-6">
              <EmptyState
                icon={<Puzzle className="w-12 h-12 text-gray-400" />}
                title="Plugin Marketplace"
                description="Browse and install plugins from the official marketplace"
                action={
                  <Button
                    className="flex items-center gap-2"
                    onClick={handleBrowseMarketplace}
                    aria-label="Browse plugin marketplace"
                  >
                    <Upload className="w-4 h-4" />
                    Browse Marketplace
                  </Button>
                }
              />
            </div>
          </Card>
        </TabsContent>
      </Tabs>

      {selectedPlugin && (
        <PluginDetails
          pluginId={selectedPlugin}
          onClose={() => setSelectedPlugin(null)}
        />
      )}

      {showInstallModal && (
        <InstallPluginModal
          onClose={() => setShowInstallModal(false)}
        />
      )}
    </div>
  );
}
