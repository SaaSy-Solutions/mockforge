import { create } from 'zustand';

// Types matching the backend API responses
export interface SummaryMetrics {
  timestamp: string;
  request_rate: number;
  p95_latency_ms: number;
  error_rate_percent: number;
  active_connections: number;
}

export interface SeriesData {
  name: string;
  values: number[];
}

export interface RequestMetrics {
  timestamps: number[];
  series: SeriesData[];
}

export interface EndpointMetrics {
  path: string;
  method: string;
  request_rate: number;
  avg_latency_ms: number;
  p95_latency_ms: number;
  errors: number;
  error_rate_percent: number;
}

export interface WebSocketMetrics {
  active_connections: number;
  total_connections: number;
  message_rate_sent: number;
  message_rate_received: number;
  error_rate: number;
  avg_connection_duration_seconds: number;
}

export interface SmtpMetrics {
  active_connections: number;
  total_connections: number;
  message_rate_received: number;
  message_rate_stored: number;
  error_rate: number;
}

export interface SystemMetrics {
  memory_usage_mb: number;
  cpu_usage_percent: number;
  thread_count: number;
  uptime_seconds: number;
}

export type TimeRange = '5m' | '15m' | '1h' | '6h' | '24h';

export interface AnalyticsStore {
  // State
  summary: SummaryMetrics | null;
  requests: RequestMetrics | null;
  endpoints: EndpointMetrics[];
  websocket: WebSocketMetrics | null;
  smtp: SmtpMetrics | null;
  system: SystemMetrics | null;
  timeRange: TimeRange;
  isLoading: boolean;
  error: string | null;
  lastUpdated: Date | null;

  // Actions
  setTimeRange: (range: TimeRange) => void;
  fetchSummary: (range: TimeRange) => Promise<void>;
  fetchRequests: (range: TimeRange) => Promise<void>;
  fetchEndpoints: (limit?: number) => Promise<void>;
  fetchWebSocket: () => Promise<void>;
  fetchSmtp: () => Promise<void>;
  fetchSystem: () => Promise<void>;
  fetchAll: (range?: TimeRange) => Promise<void>;
  clearError: () => void;
}

const BASE_URL = '__mockforge/analytics';

export const useAnalyticsStore = create<AnalyticsStore>((set, get) => ({
  // Initial state
  summary: null,
  requests: null,
  endpoints: [],
  websocket: null,
  smtp: null,
  system: null,
  timeRange: '1h',
  isLoading: false,
  error: null,
  lastUpdated: null,

  setTimeRange: (range: TimeRange) => {
    set({ timeRange: range });
    // Auto-fetch when time range changes
    get().fetchAll(range);
  },

  fetchSummary: async (range: TimeRange) => {
    try {
      set({ isLoading: true, error: null });
      const response = await fetch(`/${BASE_URL}/summary?range=${range}`);

      if (!response.ok) {
        throw new Error(`Failed to fetch summary: ${response.statusText}`);
      }

      const data = await response.json();
      set({
        summary: data.data,
        isLoading: false,
        lastUpdated: new Date()
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      set({ error: errorMessage, isLoading: false });
      console.error('Failed to fetch summary:', error);
    }
  },

  fetchRequests: async (range: TimeRange) => {
    try {
      set({ isLoading: true, error: null });
      const response = await fetch(`/${BASE_URL}/requests?range=${range}`);

      if (!response.ok) {
        throw new Error(`Failed to fetch requests: ${response.statusText}`);
      }

      const data = await response.json();
      set({
        requests: data.data,
        isLoading: false,
        lastUpdated: new Date()
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      set({ error: errorMessage, isLoading: false });
      console.error('Failed to fetch requests:', error);
    }
  },

  fetchEndpoints: async (limit = 10) => {
    try {
      set({ isLoading: true, error: null });
      const response = await fetch(`/${BASE_URL}/endpoints?limit=${limit}`);

      if (!response.ok) {
        throw new Error(`Failed to fetch endpoints: ${response.statusText}`);
      }

      const data = await response.json();
      set({
        endpoints: data.data,
        isLoading: false,
        lastUpdated: new Date()
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      set({ error: errorMessage, isLoading: false });
      console.error('Failed to fetch endpoints:', error);
    }
  },

  fetchWebSocket: async () => {
    try {
      set({ isLoading: true, error: null });
      const response = await fetch(`/${BASE_URL}/websocket`);

      if (!response.ok) {
        throw new Error(`Failed to fetch WebSocket metrics: ${response.statusText}`);
      }

      const data = await response.json();
      set({
        websocket: data.data,
        isLoading: false,
        lastUpdated: new Date()
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      set({ error: errorMessage, isLoading: false });
      console.error('Failed to fetch WebSocket metrics:', error);
    }
  },

  fetchSmtp: async () => {
    try {
      set({ isLoading: true, error: null });
      const response = await fetch(`/${BASE_URL}/smtp`);

      if (!response.ok) {
        throw new Error(`Failed to fetch SMTP metrics: ${response.statusText}`);
      }

      const data = await response.json();
      set({
        smtp: data.data,
        isLoading: false,
        lastUpdated: new Date()
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      set({ error: errorMessage, isLoading: false });
      console.error('Failed to fetch SMTP metrics:', error);
    }
  },

  fetchSystem: async () => {
    try {
      set({ isLoading: true, error: null });
      const response = await fetch(`/${BASE_URL}/system`);

      if (!response.ok) {
        throw new Error(`Failed to fetch system metrics: ${response.statusText}`);
      }

      const data = await response.json();
      set({
        system: data.data,
        isLoading: false,
        lastUpdated: new Date()
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      set({ error: errorMessage, isLoading: false });
      console.error('Failed to fetch system metrics:', error);
    }
  },

  fetchAll: async (range?: TimeRange) => {
    const timeRange = range || get().timeRange;
    set({ isLoading: true, error: null });

    try {
      await Promise.all([
        get().fetchSummary(timeRange),
        get().fetchRequests(timeRange),
        get().fetchEndpoints(),
        get().fetchWebSocket(),
        get().fetchSmtp(),
        get().fetchSystem(),
      ]);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      set({ error: errorMessage, isLoading: false });
      console.error('Failed to fetch analytics:', error);
    }
  },

  clearError: () => set({ error: null }),
}));

// Auto-refresh analytics every 10 seconds
setInterval(() => {
  const store = useAnalyticsStore.getState();
  if (!store.isLoading && !store.error) {
    store.fetchAll();
  }
}, 10000);
