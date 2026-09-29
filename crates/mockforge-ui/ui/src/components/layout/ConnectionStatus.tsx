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

import { Wifi, WifiOff, Loader2, Cloud } from 'lucide-react';
import { cn } from '../../utils/cn';
import { isCloudMode as detectCloudMode } from '../../utils/cloudMode';

export type ConnectionState = 'connected' | 'connecting' | 'disconnected' | 'reconnecting' | 'cloud';

interface ConnectionStatusProps {
  state: ConnectionState;
  className?: string;
  /** Optional label to show next to the indicator */
  showLabel?: boolean;
  /** Last successful connection time */
  lastConnected?: Date;
}

const stateConfig: Record<ConnectionState, { color: string; label: string; icon: 'wifi' | 'wifi-off' | 'loader' | 'cloud' }> = {
  connected: {
    color: 'bg-success-500',
    label: 'Live',
    icon: 'wifi',
  },
  connecting: {
    color: 'bg-warning-500',
    label: 'Connecting...',
    icon: 'loader',
  },
  reconnecting: {
    color: 'bg-warning-500',
    label: 'Reconnecting...',
    icon: 'loader',
  },
  disconnected: {
    color: 'bg-danger-500',
    label: 'Disconnected',
    icon: 'wifi-off',
  },
  cloud: {
    color: 'bg-info-500',
    label: 'Cloud',
    icon: 'cloud',
  },
};

export function ConnectionStatus({
  state,
  className,
  showLabel = false,
  lastConnected,
}: ConnectionStatusProps) {
  const config = stateConfig[state];

  const Icon = config.icon === 'wifi' ? Wifi : config.icon === 'wifi-off' ? WifiOff : config.icon === 'cloud' ? Cloud : Loader2;

  return (
    <div
      className={cn(
        'flex items-center gap-1.5',
        showLabel && 'h-7 rounded-full border border-border bg-bg-primary px-2.5',
        className
      )}
      role="status"
      aria-live="polite"
      aria-label={config.label}
      title={lastConnected ? `Last connected: ${lastConnected.toLocaleTimeString()}` : config.label}
    >
      <span className={cn('inline-flex h-2 w-2 shrink-0 rounded-full', config.color)} aria-hidden />
      {showLabel && (
        <span className="flex items-center gap-1 text-xs font-medium text-muted-foreground">
          <Icon className={cn('h-3 w-3', (state === 'connecting' || state === 'reconnecting') && 'animate-spin')} aria-hidden />
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

// In cloud mode a WebSocket to a local server is not expected. Use the shared
// detector so VITE_MOCKFORGE_MODE=cloud builds are recognised too.
const isCloudMode = detectCloudMode();

interface ConnectionStore {
  backendState: ConnectionState;
  wsState: ConnectionState;
  /** Number of active hosted mock streams connected */
  hostedMockStreams: number;
  lastBackendConnected?: Date;
  lastWsConnected?: Date;
  setBackendState: (state: ConnectionState) => void;
  setWsState: (state: ConnectionState) => void;
  incrementHostedMockStreams: () => void;
  decrementHostedMockStreams: () => void;
}

export const useConnectionStore = create<ConnectionStore>((set) => ({
  backendState: isCloudMode ? 'connected' : 'connecting',
  wsState: isCloudMode ? 'cloud' : 'disconnected',
  hostedMockStreams: 0,
  setBackendState: (state) => set({
    backendState: state,
    lastBackendConnected: state === 'connected' ? new Date() : undefined,
  }),
  setWsState: (state) => set({
    wsState: state,
    lastWsConnected: state === 'connected' ? new Date() : undefined,
  }),
  incrementHostedMockStreams: () => set((s) => ({ hostedMockStreams: s.hostedMockStreams + 1 })),
  decrementHostedMockStreams: () => set((s) => ({ hostedMockStreams: Math.max(0, s.hostedMockStreams - 1) })),
}));

/**
 * GlobalConnectionStatus - Shows overall connection health
 *
 * In cloud mode the local WebSocket is intentionally disabled, so we
 * show a neutral "Cloud" indicator instead of a misleading red
 * "Disconnected". When any hosted-mock stream is active we upgrade
 * the indicator to green "Connected".
 */
export function GlobalConnectionStatus({ className }: { className?: string }) {
  const { backendState, wsState, hostedMockStreams } = useConnectionStore();

  // In cloud mode, the wsState starts as 'cloud'. If we have active
  // hosted mock streams, treat the WS layer as connected.
  const effectiveWsState: ConnectionState =
    wsState === 'cloud' && hostedMockStreams > 0 ? 'connected' : wsState;

  // Determine overall status (worst of the two, but 'cloud' is neutral)
  const overallState: ConnectionState =
    backendState === 'disconnected' || effectiveWsState === 'disconnected'
      ? 'disconnected'
      : backendState === 'connecting' || effectiveWsState === 'connecting'
      ? 'connecting'
      : backendState === 'reconnecting' || effectiveWsState === 'reconnecting'
      ? 'reconnecting'
      : effectiveWsState === 'cloud'
      ? 'cloud'
      : 'connected';

  return (
    <ConnectionStatus
      state={overallState}
      className={className}
      showLabel
    />
  );
}
