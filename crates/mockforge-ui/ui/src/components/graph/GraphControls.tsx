import React, { useState } from 'react';
import { Button } from '../ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { Label } from '../ui/label';
import { RefreshCw, Download, Filter, Layout, X } from 'lucide-react';
import { Badge } from '../ui/Badge';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '../ui/Dialog';

export type LayoutType = 'hierarchical' | 'force-directed' | 'grid' | 'circular';
export type FilterType = 'all' | 'endpoint' | 'service' | 'workspace';
export type ProtocolFilter = 'all' | 'http' | 'grpc' | 'websocket' | 'graphql' | 'mqtt' | 'kafka' | 'amqp' | 'ftp' | 'smtp' | 'tcp';

interface GraphControlsProps {
  layout: LayoutType;
  onLayoutChange: (layout: LayoutType) => void;
  nodeFilter: FilterType;
  onNodeFilterChange: (filter: FilterType) => void;
  protocolFilter: ProtocolFilter;
  onProtocolFilterChange: (filter: ProtocolFilter) => void;
  onRefresh: () => void;
  onExport: (format: 'png' | 'svg' | 'json') => void;
  nodeCount: number;
  edgeCount: number;
}

export function GraphControls({
  layout,
  onLayoutChange,
  nodeFilter,
  onNodeFilterChange,
  protocolFilter,
  onProtocolFilterChange,
  onRefresh,
  onExport,
  nodeCount,
  edgeCount,
}: GraphControlsProps) {
  const [filtersOpen, setFiltersOpen] = useState(false);

  return (
    <div className="flex items-center justify-between p-4 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
      <div className="flex items-center gap-4">
        {/* Layout Selector */}
        <div className="flex items-center gap-2">
          <Layout className="h-4 w-4 text-gray-500" />
          <Label htmlFor="layout" className="text-sm font-medium">
            Layout:
          </Label>
          <Select value={layout} onValueChange={(value) => onLayoutChange(value as LayoutType)}>
            <SelectTrigger id="layout" className="w-[180px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="hierarchical">Hierarchical</SelectItem>
              <SelectItem value="force-directed">Force-Directed</SelectItem>
              <SelectItem value="grid">Grid</SelectItem>
              <SelectItem value="circular">Circular</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {/* Filters */}
        <Dialog open={filtersOpen} onOpenChange={setFiltersOpen}>
          <DialogTrigger asChild>
            <Button variant="outline" size="sm">
              <Filter className="h-4 w-4 mr-2" />
              Filters
              {(nodeFilter !== 'all' || protocolFilter !== 'all') && (
                <Badge variant="secondary" className="ml-2">
                  2
                </Badge>
              )}
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Graph Filters</DialogTitle>
              <DialogDescription>
                Filter nodes and edges by type and protocol
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div>
                <Label htmlFor="node-filter">Node Type</Label>
                <Select
                  value={nodeFilter}
                  onValueChange={(value) => onNodeFilterChange(value as FilterType)}
                >
                  <SelectTrigger id="node-filter" className="w-full mt-2">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Types</SelectItem>
                    <SelectItem value="endpoint">Endpoints</SelectItem>
                    <SelectItem value="service">Services</SelectItem>
                    <SelectItem value="workspace">Workspaces</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div>
                <Label htmlFor="protocol-filter">Protocol</Label>
                <Select
                  value={protocolFilter}
                  onValueChange={(value) => onProtocolFilterChange(value as ProtocolFilter)}
                >
                  <SelectTrigger id="protocol-filter" className="w-full mt-2">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Protocols</SelectItem>
                    <SelectItem value="http">HTTP</SelectItem>
                    <SelectItem value="grpc">gRPC</SelectItem>
                    <SelectItem value="websocket">WebSocket</SelectItem>
                    <SelectItem value="graphql">GraphQL</SelectItem>
                    <SelectItem value="mqtt">MQTT</SelectItem>
                    <SelectItem value="kafka">Kafka</SelectItem>
                    <SelectItem value="amqp">AMQP</SelectItem>
                    <SelectItem value="smtp">SMTP</SelectItem>
                    <SelectItem value="ftp">FTP</SelectItem>
                    <SelectItem value="tcp">TCP</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              {(nodeFilter !== 'all' || protocolFilter !== 'all') && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    onNodeFilterChange('all');
                    onProtocolFilterChange('all');
                  }}
                  className="w-full"
                >
                  <X className="h-4 w-4 mr-2" />
                  Clear Filters
                </Button>
              )}
            </div>
          </DialogContent>
        </Dialog>

        {/* Stats */}
        <div className="text-sm text-gray-600 dark:text-gray-400">
          <span className="font-medium">{nodeCount}</span> nodes,{' '}
          <span className="font-medium">{edgeCount}</span> edges
        </div>
      </div>

      <div className="flex items-center gap-2">
        {/* Export */}
        <div className="relative group">
          <Button variant="outline" size="sm">
            <Download className="h-4 w-4 mr-2" />
            Export
          </Button>
          <div className="absolute right-0 mt-2 w-32 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-50">
            <button
              onClick={() => onExport('png')}
              className="w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 rounded-t-md"
            >
              PNG
            </button>
            <button
              onClick={() => onExport('svg')}
              className="w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700"
            >
              SVG
            </button>
            <button
              onClick={() => onExport('json')}
              className="w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 rounded-b-md"
            >
              JSON
            </button>
          </div>
        </div>

        {/* Refresh */}
        <Button variant="outline" size="sm" onClick={onRefresh}>
          <RefreshCw className="h-4 w-4 mr-2" />
          Refresh
        </Button>
      </div>
    </div>
  );
}
