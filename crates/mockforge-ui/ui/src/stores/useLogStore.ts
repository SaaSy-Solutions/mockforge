import { logger } from '@/utils/logger';
import { create } from 'zustand';
import type { RequestLog, LogFilter } from '../types';

interface LogStore {
  logs: RequestLog[];
  filteredLogs: RequestLog[];
  selectedLog: RequestLog | null;
  filter: LogFilter;
  autoScroll: boolean;
  isPaused: boolean;
  connectionStatus: 'connected' | 'disconnected' | 'connecting';

  setLogs: (logs: RequestLog[]) => void;
  addLog: (log: RequestLog) => void;
  selectLog: (log: RequestLog | null) => void;
  setFilter: (filter: Partial<LogFilter>) => void;
  clearFilter: () => void;
  setAutoScroll: (enabled: boolean) => void;
  setPaused: (paused: boolean) => void;
  setConnectionStatus: (status: 'connected' | 'disconnected' | 'connecting') => void;
  applyFilter: () => void;
  clearLogs: () => void;
  startLogStream: () => void;
  stopLogStream: () => void;
}

// Mock log data generator
const generateMockLog = (id: number): RequestLog => {
  const methods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'];
  const paths = [
    '/api/users',
    '/api/users/123',
    '/api/orders',
    '/api/orders/456',
    '/api/products',
    '/api/auth/login',
    '/api/auth/logout',
    '/api/webhooks/stripe',
    '/health',
    '/metrics'
  ];
  const statusCodes = [200, 201, 204, 400, 401, 403, 404, 422, 500, 502];
  const userAgents = [
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
    'curl/7.68.0',
    'PostmanRuntime/7.28.4',
    'MockForge/1.0.0',
  ];
  const ips = ['192.168.1.100', '10.0.0.50', '172.16.0.25', '203.0.113.1'];

  const method = methods[Math.floor(Math.random() * methods.length)];
  const path = paths[Math.floor(Math.random() * paths.length)];
  const statusCode = statusCodes[Math.floor(Math.random() * statusCodes.length)];
  const responseTime = Math.floor(Math.random() * 2000) + 10;
  const responseSize = Math.floor(Math.random() * 10000) + 100;
  const hasError = statusCode >= 400 && Math.random() < 0.3;

  return {
    id: `req-${id}-${Date.now()}`,
    timestamp: new Date().toISOString(),
    method,
    path,
    status_code: statusCode,
    response_time_ms: responseTime,
    client_ip: ips[Math.floor(Math.random() * ips.length)],
    user_agent: userAgents[Math.floor(Math.random() * userAgents.length)],
    headers: {
      'Content-Type': 'application/json',
      'Accept': 'application/json',
      'Authorization': 'Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...',
      'X-Request-ID': `req-${id}`,
      'User-Agent': userAgents[Math.floor(Math.random() * userAgents.length)],
    },
    request_size_bytes: Math.floor(Math.random() * 1000) + 50,
    response_size_bytes: responseSize,
    error_message: hasError ? `${statusCode === 404 ? 'Resource not found' : statusCode === 500 ? 'Internal server error' : 'Bad request'}` : undefined,
  };
};

// Generate initial mock logs
const initialLogs = Array.from({ length: 50 }, (_, i) => generateMockLog(i + 1));

const defaultFilter: LogFilter = {
  hours_ago: 24,
  limit: 100,
};

const applyLogFilter = (logs: RequestLog[], filter: LogFilter): RequestLog[] => {
  let filtered = logs;

  // Filter by method
  if (filter.method) {
    filtered = filtered.filter(log => log.method === filter.method);
  }

  // Filter by status code
  if (filter.status_code) {
    filtered = filtered.filter(log => log.status_code === filter.status_code);
  }

  // Filter by path pattern (search)
  if (filter.path_pattern) {
    const pattern = filter.path_pattern.toLowerCase();
    filtered = filtered.filter(log => 
      log.path.toLowerCase().includes(pattern) ||
      log.method.toLowerCase().includes(pattern) ||
      (log.error_message && log.error_message.toLowerCase().includes(pattern))
    );
  }

  // Filter by time range
  if (filter.hours_ago) {
    const cutoff = new Date();
    cutoff.setHours(cutoff.getHours() - filter.hours_ago);
    filtered = filtered.filter(log => new Date(log.timestamp) >= cutoff);
  }

  // Apply limit
  if (filter.limit) {
    filtered = filtered.slice(-filter.limit);
  }

  return filtered.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());
};

// Log stream interval management
let logStreamInterval: ReturnType<typeof setInterval> | null = null;
let logCounter = 51;

export const useLogStore = create<LogStore>((set, get) => ({
  logs: initialLogs,
  filteredLogs: applyLogFilter(initialLogs, defaultFilter),
  selectedLog: null,
  filter: defaultFilter,
  autoScroll: true,
  isPaused: false,
  connectionStatus: 'connected',

  setLogs: (logs) => {
    const filteredLogs = applyLogFilter(logs, get().filter);
    set({ logs, filteredLogs });
  },

  addLog: (log) => {
    const state = get();
    if (state.isPaused) return;
    
    const newLogs = [...state.logs, log];
    const filteredLogs = applyLogFilter(newLogs, state.filter);
    set({ logs: newLogs, filteredLogs });
  },

  selectLog: (log) => set({ selectedLog: log }),

  setFilter: (newFilter) => {
    const state = get();
    const updatedFilter = { ...state.filter, ...newFilter };
    const filteredLogs = applyLogFilter(state.logs, updatedFilter);
    set({ filter: updatedFilter, filteredLogs });
  },

  clearFilter: () => {
    const state = get();
    const clearedFilter = { hours_ago: 24, limit: 100 };
    const filteredLogs = applyLogFilter(state.logs, clearedFilter);
    set({ filter: clearedFilter, filteredLogs });
  },

  setAutoScroll: (enabled) => set({ autoScroll: enabled }),

  setPaused: (paused) => set({ isPaused: paused }),

  setConnectionStatus: (status) => set({ connectionStatus: status }),

  applyFilter: () => {
    const state = get();
    const filteredLogs = applyLogFilter(state.logs, state.filter);
    set({ filteredLogs });
  },

  clearLogs: () => set({ logs: [], filteredLogs: [], selectedLog: null }),

  startLogStream: () => {
    // Clear any existing interval
    if (logStreamInterval) {
      clearInterval(logStreamInterval);
    }

    // Start new interval
    logStreamInterval = setInterval(() => {
      const store = get();
      if (!store.isPaused && store.connectionStatus === 'connected') {
        store.addLog(generateMockLog(logCounter++));
      }
    }, 2000 + Math.random() * 3000); // Random interval between 2-5 seconds
  },

  stopLogStream: () => {
    if (logStreamInterval) {
      clearInterval(logStreamInterval);
      logStreamInterval = null;
    }
  },
}));