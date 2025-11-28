/**
 * Tauri API wrapper for desktop app integration
 *
 * This module provides type-safe wrappers around Tauri commands.
 * In the web version, these functions will gracefully degrade or use
 * the REST API instead.
 */

// Check if running in Tauri
export const isTauri =
  typeof window !== 'undefined' &&
  ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);

/**
 * Server status from Tauri backend
 */
export interface ServerStatus {
  running: boolean;
  http_port?: number;
  admin_port?: number;
  error?: string;
}

/**
 * Start the mock server
 */
export async function startServer(
  configPath?: string,
  httpPort?: number,
  adminPort?: number
): Promise<ServerStatus> {
  if (isTauri) {
    try {
      const { invoke } = await import('@tauri-apps/api/tauri');
      return await invoke<ServerStatus>('start_server', {
        config_path: configPath,
        http_port: httpPort,
        admin_port: adminPort,
      });
    } catch (error) {
      return {
        running: false,
        error: error instanceof Error ? error.message : 'Failed to start server',
      };
    }
  } else {
    // Web version - server should already be running
    // Return current status
    return {
      running: true,
      http_port: 3000,
      admin_port: 9080,
    };
  }
}

/**
 * Stop the mock server
 */
export async function stopServer(): Promise<ServerStatus> {
  if (isTauri) {
    try {
      const { invoke } = await import('@tauri-apps/api/tauri');
      return await invoke<ServerStatus>('stop_server');
    } catch (error) {
      return {
        running: false,
        error: error instanceof Error ? error.message : 'Failed to stop server',
      };
    }
  } else {
    // Web version - can't stop server from UI
    return {
      running: true,
      error: 'Server control not available in web version',
    };
  }
}

/**
 * Get current server status
 */
export async function getServerStatus(): Promise<ServerStatus> {
  if (isTauri) {
    try {
      const { invoke } = await import('@tauri-apps/api/tauri');
      return await invoke<ServerStatus>('get_server_status');
    } catch (error) {
      return {
        running: false,
        error: error instanceof Error ? error.message : 'Failed to get server status',
      };
    }
  } else {
    // Web version - check if server is reachable
    try {
      const response = await fetch('http://localhost:9080/health/live');
      return {
        running: response.ok,
        http_port: 3000,
        admin_port: 9080,
      };
    } catch {
      return {
        running: false,
        http_port: 3000,
        admin_port: 9080,
        error: 'Server not reachable',
      };
    }
  }
}

/**
 * Open a configuration file
 */
export async function openConfigFile(): Promise<string | null> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/tauri');
    return invoke<string | null>('open_config_file');
  } else {
    // Web version - use file input
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.yaml,.yml,.json';
      input.onchange = async (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (file) {
          const text = await file.text();
          resolve(text);
        } else {
          resolve(null);
        }
      };
      input.click();
    });
  }
}

/**
 * Save a configuration file
 */
export async function saveConfigFile(content: string): Promise<string | null> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/tauri');
    return invoke<string | null>('save_config_file', { content });
  } else {
    // Web version - download file
    const blob = new Blob([content], { type: 'application/yaml' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'mockforge.yaml';
    a.click();
    URL.revokeObjectURL(url);
    return 'mockforge.yaml';
  }
}

/**
 * Get app version
 */
export async function getAppVersion(): Promise<string> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/tauri');
    return invoke<string>('get_app_version');
  } else {
    return 'web';
  }
}

/**
 * Listen for Tauri events
 */
export function listenToTauriEvent<T = unknown>(
  event: string,
  handler: (payload: T) => void
): () => void {
  if (isTauri) {
    let unlisten: (() => void) | null = null;
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen<T>(event, (event) => {
        handler(event.payload);
      }).then((unlistenFn) => {
        unlisten = unlistenFn;
      });
    });
    // Return cleanup function
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }
  return () => {};
}

/**
 * Handle file open event (from file association)
 */
export async function handleFileOpen(filePath: string): Promise<void> {
  if (isTauri) {
    try {
      const { invoke } = await import('@tauri-apps/api/tauri');
      await invoke('handle_file_open', { file_path: filePath });
    } catch (error) {
      console.error('Failed to handle file open:', error);
    }
  }
}
