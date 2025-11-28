import { logger } from '@/utils/logger';
import React, { useState, useEffect, useMemo } from 'react';
import { Play, Search, Filter, Clock, X, CheckCircle, XCircle, AlertCircle } from 'lucide-react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { Badge } from '../ui/Badge';
import { usePlaygroundStore, type PlaygroundHistoryEntry } from '../../stores/usePlaygroundStore';
import { toast } from 'sonner';

/**
 * History Panel Component
 *
 * Displays request history with:
 * - List of previous requests
 * - Filter by protocol, status, date
 * - Search functionality
 * - One-click replay
 */
export function HistoryPanel() {
  const { history, historyLoading, loadHistory, replayRequest } = usePlaygroundStore();
  const [searchQuery, setSearchQuery] = useState('');
  const [protocolFilter, setProtocolFilter] = useState<string>('all');
  const [statusFilter, setStatusFilter] = useState<string>('all');

  // Load history on mount
  useEffect(() => {
    loadHistory({ limit: 50 });
  }, [loadHistory]);

  // Filtered history
  const filteredHistory = useMemo(() => {
    return history.filter((entry) => {
      // Search filter
      if (searchQuery) {
        const query = searchQuery.toLowerCase();
        const matchesSearch =
          entry.path.toLowerCase().includes(query) ||
          entry.method.toLowerCase().includes(query) ||
          (entry.graphql_query?.toLowerCase().includes(query) ?? false);
        if (!matchesSearch) return false;
      }

      // Protocol filter
      if (protocolFilter !== 'all' && entry.protocol !== protocolFilter) {
        return false;
      }

      // Status filter
      if (statusFilter !== 'all') {
        if (statusFilter === 'success' && entry.status_code >= 400) {
          return false;
        }
        if (statusFilter === 'error' && entry.status_code < 400) {
          return false;
        }
      }

      return true;
    });
  }, [history, searchQuery, protocolFilter, statusFilter]);

  // Handle replay
  const handleReplay = async (requestId: string) => {
    try {
      await replayRequest(requestId);
      toast.success('Request replayed');
    } catch (error) {
      logger.error('Failed to replay request', error);
      toast.error('Failed to replay request');
    }
  };

  // Format timestamp
  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  };

  // Get status icon
  const getStatusIcon = (status: number) => {
    if (status >= 200 && status < 300) {
      return <CheckCircle className="h-4 w-4 text-green-600" />;
    }
    if (status >= 400) {
      return <XCircle className="h-4 w-4 text-red-600" />;
    }
    return <AlertCircle className="h-4 w-4 text-yellow-600" />;
  };

  // Get status color
  const getStatusColor = (status: number) => {
    if (status >= 200 && status < 300) return 'bg-green-500';
    if (status >= 300 && status < 400) return 'bg-blue-500';
    if (status >= 400 && status < 500) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  return (
    <Card className="h-full flex flex-col">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg font-semibold">History</CardTitle>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              setSearchQuery('');
              setProtocolFilter('all');
              setStatusFilter('all');
            }}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>

      <CardContent className="flex-1 overflow-auto space-y-4">
        {/* Filters */}
        <div className="space-y-2">
          <div className="relative">
            <Search className="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search requests..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8"
            />
          </div>
          <div className="grid grid-cols-2 gap-2">
            <Select value={protocolFilter} onValueChange={setProtocolFilter}>
              <SelectTrigger>
                <SelectValue placeholder="Protocol" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Protocols</SelectItem>
                <SelectItem value="rest">REST</SelectItem>
                <SelectItem value="graphql">GraphQL</SelectItem>
              </SelectContent>
            </Select>
            <Select value={statusFilter} onValueChange={setStatusFilter}>
              <SelectTrigger>
                <SelectValue placeholder="Status" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">All Status</SelectItem>
                <SelectItem value="success">Success</SelectItem>
                <SelectItem value="error">Error</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        {/* History List */}
        {historyLoading ? (
          <div className="flex items-center justify-center py-8">
            <div className="text-center space-y-2">
              <div className="inline-block animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
              <p className="text-sm text-muted-foreground">Loading history...</p>
            </div>
          </div>
        ) : filteredHistory.length === 0 ? (
          <div className="flex items-center justify-center py-8">
            <div className="text-center space-y-2">
              <Clock className="h-8 w-8 mx-auto text-muted-foreground" />
              <p className="text-sm text-muted-foreground">No requests found</p>
            </div>
          </div>
        ) : (
          <div className="space-y-2">
            {filteredHistory.map((entry) => (
              <div
                key={entry.id}
                className="border rounded-md p-3 hover:bg-muted/50 transition-colors cursor-pointer"
                onClick={() => handleReplay(entry.id)}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <Badge variant="outline" className="text-xs">
                        {entry.protocol.toUpperCase()}
                      </Badge>
                      <Badge variant="outline" className="text-xs">
                        {entry.method}
                      </Badge>
                      <Badge className={`${getStatusColor(entry.status_code)} text-xs`}>
                        {entry.status_code}
                      </Badge>
                      {getStatusIcon(entry.status_code)}
                    </div>
                    <div className="text-sm font-mono truncate">
                      {entry.protocol === 'graphql' && entry.graphql_query
                        ? entry.graphql_query.substring(0, 60) + '...'
                        : entry.path}
                    </div>
                    <div className="flex items-center gap-4 mt-2 text-xs text-muted-foreground">
                      <div className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        {entry.response_time_ms}ms
                      </div>
                      <div>{formatTimestamp(entry.timestamp)}</div>
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="flex-shrink-0"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleReplay(entry.id);
                    }}
                  >
                    <Play className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
