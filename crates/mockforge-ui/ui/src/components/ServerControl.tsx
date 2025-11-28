/**
 * Server Control Component
 *
 * Provides UI for starting/stopping the embedded mock server in desktop app.
 * In web version, shows server status only.
 */

import { useState, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getServerStatus,
  startServer,
  stopServer,
  isTauri,
  type ServerStatus,
} from '@/utils/tauri';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Loader2, Play, Square, RefreshCw } from 'lucide-react';

export function ServerControl() {
  const queryClient = useQueryClient();
  const [autoRefresh, setAutoRefresh] = useState(true);

  // Query server status
  const { data: status, isLoading, refetch } = useQuery<ServerStatus>({
    queryKey: ['server-status'],
    queryFn: getServerStatus,
    refetchInterval: autoRefresh ? 2000 : false, // Refresh every 2 seconds if enabled
    refetchOnWindowFocus: true,
  });

  // Start server mutation
  const startMutation = useMutation({
    mutationFn: (args?: { configPath?: string; httpPort?: number; adminPort?: number }) =>
      startServer(args?.configPath, args?.httpPort, args?.adminPort),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['server-status'] });
    },
  });

  // Stop server mutation
  const stopMutation = useMutation({
    mutationFn: stopServer,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['server-status'] });
    },
  });

  // Listen for Tauri events
  useEffect(() => {
    if (!isTauri) return;

    const cleanup1 = import('@tauri-apps/api/event').then(({ listen }) => {
      return listen('server-started', () => {
        queryClient.invalidateQueries({ queryKey: ['server-status'] });
      });
    });

    const cleanup2 = import('@tauri-apps/api/event').then(({ listen }) => {
      return listen('server-stopped', () => {
        queryClient.invalidateQueries({ queryKey: ['server-status'] });
      });
    });

    return () => {
      cleanup1.then((unlisten) => unlisten());
      cleanup2.then((unlisten) => unlisten());
    };
  }, [queryClient]);

  const handleStart = () => {
    startMutation.mutate();
  };

  const handleStop = () => {
    stopMutation.mutate();
  };

  if (!isTauri) {
    // Web version - just show status
    return (
      <Card>
        <CardHeader>
          <CardTitle>Server Status</CardTitle>
          <CardDescription>
            Server is managed externally in web version
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-2">
            <Badge variant={status?.running ? 'default' : 'destructive'}>
              {status?.running ? 'Running' : 'Stopped'}
            </Badge>
            {status?.http_port && (
              <span className="text-sm text-muted-foreground">
                HTTP: {status.http_port}
              </span>
            )}
            {status?.admin_port && (
              <span className="text-sm text-muted-foreground">
                Admin: {status.admin_port}
              </span>
            )}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Server Control</CardTitle>
            <CardDescription>
              Manage the embedded mock server
            </CardDescription>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => refetch()}
            disabled={isLoading}
          >
            <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {/* Status */}
          <div className="flex items-center gap-2">
            <Badge variant={status?.running ? 'default' : 'destructive'}>
              {status?.running ? 'Running' : 'Stopped'}
            </Badge>
            {status?.http_port && (
              <span className="text-sm text-muted-foreground">
                HTTP: {status.http_port}
              </span>
            )}
            {status?.admin_port && (
              <span className="text-sm text-muted-foreground">
                Admin: {status.admin_port}
              </span>
            )}
          </div>

          {/* Error message */}
          {status?.error && (
            <div className="text-sm text-destructive">{status.error}</div>
          )}

          {/* Controls */}
          <div className="flex gap-2">
            {status?.running ? (
              <Button
                onClick={handleStop}
                disabled={stopMutation.isPending}
                variant="destructive"
              >
                {stopMutation.isPending ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Stopping...
                  </>
                ) : (
                  <>
                    <Square className="mr-2 h-4 w-4" />
                    Stop Server
                  </>
                )}
              </Button>
            ) : (
              <Button
                onClick={handleStart}
                disabled={startMutation.isPending}
              >
                {startMutation.isPending ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Starting...
                  </>
                ) : (
                  <>
                    <Play className="mr-2 h-4 w-4" />
                    Start Server
                  </>
                )}
              </Button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
