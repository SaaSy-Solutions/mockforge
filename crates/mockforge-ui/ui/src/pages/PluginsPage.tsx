import React, { useState } from 'react';
import {
  Puzzle,
  Plus,
  RefreshCw,
  Upload,
} from 'lucide-react';
import {
  PageHeader,
  Button,
  Card,
  Input,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  EmptyState
} from '../components/ui/DesignSystem';
import { PluginList } from '../components/plugins/PluginList';
import { PluginDetails } from '../components/plugins/PluginDetails';
import { PluginStatus } from '../components/plugins/PluginStatus';
import { InstallPluginModal } from '../components/plugins/InstallPluginModal';

export function PluginsPage() {
  const [activeTab, setActiveTab] = useState('installed');
  const [selectedPlugin, setSelectedPlugin] = useState<string | null>(null);
  const [showInstallModal, setShowInstallModal] = useState(false);
  const [filterType, setFilterType] = useState<string>('');
  const [filterStatus, setFilterStatus] = useState<string>('');

  return (
    <div className="space-y-8">
      <PageHeader
        title="Plugin Management"
        subtitle="Manage authentication, template, and response plugins"
        action={
          <div className="flex gap-3">
            <Button
              variant="outline"
              onClick={() => setShowInstallModal(true)}
              className="flex items-center gap-2"
            >
              <Plus className="w-4 h-4" />
              Install Plugin
            </Button>
            <Button variant="outline" className="flex items-center gap-2">
              <RefreshCw className="w-4 h-4" />
              Reload All
            </Button>
          </div>
        }
      />

      <div className="flex gap-4 mb-6">
        <div className="flex-1">
          <Input
            placeholder="Filter by type (auth, template, response, datasource)"
            value={filterType}
            onChange={(e) => setFilterType(e.target.value)}
          />
        </div>
        <div className="flex-1">
          <Input
            placeholder="Filter by status"
            value={filterStatus}
            onChange={(e) => setFilterStatus(e.target.value)}
          />
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList>
          <TabsTrigger value="installed">Installed Plugins</TabsTrigger>
          <TabsTrigger value="status">System Status</TabsTrigger>
          <TabsTrigger value="marketplace">Marketplace</TabsTrigger>
        </TabsList>

        <TabsContent value="installed" className="space-y-6">
          <PluginList
            filterType={filterType}
            filterStatus={filterStatus}
            onSelectPlugin={setSelectedPlugin}
          />
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
                  <Button className="flex items-center gap-2">
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
