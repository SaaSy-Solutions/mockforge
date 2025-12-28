/**
 * ConnectionStatus - Global connection status indicator
 *
 * Shows the status of WebSocket/backend connections in the app header.
 * Features:
 * - Green dot for connected
 * - Yellow dot for connecting/reconnecting
 * - Red dot for disconnected
 * - Tooltip with detailed status
 */

import React from 'react';
import { Wifi, WifiOff, Loader2 } from 'lucide-react';
import { cn } from '../../utils/cn';

export type ConnectionState = 'connected' | 'connecting' | 'disconnected' | 'reconnecting';

interface ConnectionStatusProps {
  state: ConnectionState;
  className?: string;
  /** Optional label to show next to the indicator */
  showLabel?: boolean;
  /** Last successful connection time */
  lastConnected?: Date;
}

const stateConfig: Record<ConnectionState, { color: string; label: string; icon: 'wifi' | 'wifi-off' | 'loader' }> = {
  connected: {
    color: 'bg-green-500',
    label: 'Connected',
    icon: 'wifi',
  },
  connecting: {
    color: 'bg-yellow-500',
    label: 'Connecting...',
    icon: 'loader',
  },
  reconnecting: {
    color: 'bg-yellow-500',
    label: 'Reconnecting...',
    icon: 'loader',
  },
  disconnected: {
    color: 'bg-red-500',
    label: 'Disconnected',
    icon: 'wifi-off',
  },
};

export function ConnectionStatus({
  state,
  className,
  showLabel = false,
  lastConnected,
}: ConnectionStatusProps) {
  const config = stateConfig[state];

  const Icon = config.icon === 'wifi' ? Wifi : config.icon === 'wifi-off' ? WifiOff : Loader2;

  return (
    <div
      className={cn(
        'flex items-center gap-2',
        className
      )}
      role="status"
      aria-live="polite"
      title={lastConnected ? `Last connected: ${lastConnected.toLocaleTimeString()}` : config.label}
    >
      <span className="relative flex h-2.5 w-2.5">
        {state === 'connected' && (
          <span className={cn('animate-ping absolute inline-flex h-full w-full rounded-full opacity-75', config.color)} />
        )}
        <span className={cn('relative inline-flex rounded-full h-2.5 w-2.5', config.color)} />
      </span>
      {showLabel && (
        <span className="text-xs text-gray-600 dark:text-gray-400 flex items-center gap-1">
          <Icon className={cn('h-3 w-3', (state === 'connecting' || state === 'reconnecting') && 'animate-spin')} />
          {config.label}
        </span>
      )}
    </div>
  );
}

/**
 * Hook to get the global connection status
 * This can be extended to track multiple connections
 */
import { create } from 'zustand';

interface ConnectionStore {
  backendState: ConnectionState;
  wsState: ConnectionState;
  lastBackendConnected?: Date;
  lastWsConnected?: Date;
  setBackendState: (state: ConnectionState) => void;
  setWsState: (state: ConnectionState) => void;
}

export const useConnectionStore = create<ConnectionStore>((set) => ({
  backendState: 'connecting',
  wsState: 'disconnected',
  setBackendState: (state) => set({
    backendState: state,
    lastBackendConnected: state === 'connected' ? new Date() : undefined,
  }),
  setWsState: (state) => set({
    wsState: state,
    lastWsConnected: state === 'connected' ? new Date() : undefined,
  }),
}));

/**
 * GlobalConnectionStatus - Shows overall connection health
 */
export function GlobalConnectionStatus({ className }: { className?: string }) {
  const { backendState, wsState } = useConnectionStore();

  // Determine overall status (worst of the two)
  const overallState: ConnectionState =
    backendState === 'disconnected' || wsState === 'disconnected'
      ? 'disconnected'
      : backendState === 'connecting' || wsState === 'connecting'
      ? 'connecting'
      : backendState === 'reconnecting' || wsState === 'reconnecting'
      ? 'reconnecting'
      : 'connected';

  return (
    <ConnectionStatus
      state={overallState}
      className={className}
      showLabel={overallState !== 'connected'}
    />
  );
}
