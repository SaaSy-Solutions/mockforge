import { create } from 'zustand';
import type { LatencyMetrics, FailureMetrics, HistogramBucket } from '../types';

interface MetricsStore {
  latencyMetrics: LatencyMetrics[];
  failureMetrics: FailureMetrics[];
  selectedService: string | null;
  isLoading: boolean;
  lastUpdated: Date | null;
  
  setLatencyMetrics: (metrics: LatencyMetrics[]) => void;
  setFailureMetrics: (metrics: FailureMetrics[]) => void;
  setSelectedService: (service: string | null) => void;
  setLoading: (loading: boolean) => void;
  refreshMetrics: () => Promise<void>;
}

// Mock data generators
const generateLatencyHistogram = (service: string): HistogramBucket[] => {
  const ranges = [
    '0-50ms', '50-100ms', '100-200ms', '200-500ms', 
    '500ms-1s', '1s-2s', '2s-5s', '5s+'
  ];
  
  return ranges.map((range, index) => {
    // Simulate realistic distribution - most requests fast, fewer slow
    let count;
    if (index < 3) count = Math.floor(Math.random() * 500) + 200; // Fast requests
    else if (index < 5) count = Math.floor(Math.random() * 100) + 50; // Medium requests
    else count = Math.floor(Math.random() * 20) + 5; // Slow requests
    
    const total = 1000;
    return {
      range,
      le: index * 100, // Convert to numeric value for le
      count,
      percentage: (count / total) * 100,
    };
  });
};

const generateLatencyMetrics = (): LatencyMetrics[] => {
  const services = [
    { name: 'user-service', route: '/api/users' },
    { name: 'order-service', route: '/api/orders' },
    { name: 'payment-service', route: '/api/payments' },
    { name: 'inventory-grpc', route: 'inventory.InventoryService' },
    { name: 'notification-service', route: '/api/notifications' },
  ];

  return services.map(service => {
    const avg = Math.floor(Math.random() * 150) + 50;
    const min = Math.floor(Math.random() * 30) + 10;
    const max = Math.floor(Math.random() * 1000) + 500;
    const p50 = Math.floor(Math.random() * 100) + 30;
    const totalRequests = Math.floor(Math.random() * 5000) + 1000;
    return {
      service: service.name,
      route: service.route,
      total_requests: totalRequests,
      avg_response_time: avg,
      min_response_time: min,
      max_response_time: max,
      p50_response_time: p50,
      p95_response_time: Math.floor(Math.random() * 300) + 200,
      p99_response_time: Math.floor(Math.random() * 800) + 500,
      p50: p50,
      p95: Math.floor(Math.random() * 300) + 200,
      p99: Math.floor(Math.random() * 800) + 500,
      histogram: generateLatencyHistogram(service.name),
    };
  });
};

const generateFailureMetrics = (): FailureMetrics[] => {
  const services = [
    'user-service',
    'order-service', 
    'payment-service',
    'inventory-grpc',
    'notification-service',
  ];

  return services.map(service => {
    const totalRequests = Math.floor(Math.random() * 5000) + 1000;
    const failureCount = Math.floor(Math.random() * totalRequests * 0.1); // 0-10% failure rate
    const successCount = totalRequests - failureCount;
    
    // Generate realistic status code distribution
    const statusCodes: Record<number, number> = {};
    
    // Success codes
    statusCodes[200] = Math.floor(successCount * 0.8);
    statusCodes[201] = Math.floor(successCount * 0.15);
    statusCodes[204] = successCount - statusCodes[200] - statusCodes[201];
    
    // Error codes
    if (failureCount > 0) {
      statusCodes[400] = Math.floor(failureCount * 0.3);
      statusCodes[401] = Math.floor(failureCount * 0.1);
      statusCodes[403] = Math.floor(failureCount * 0.1);
      statusCodes[404] = Math.floor(failureCount * 0.2);
      statusCodes[422] = Math.floor(failureCount * 0.1);
      statusCodes[500] = Math.floor(failureCount * 0.15);
      statusCodes[502] = Math.floor(failureCount * 0.03);
      statusCodes[503] = failureCount - Object.values(statusCodes).reduce((sum, count) => sum + count, 0) + totalRequests - successCount;
    }

    return {
      service,
      total_requests: totalRequests,
      success_count: successCount,
      failure_count: failureCount,
      total_errors: failureCount,
      error_rate: failureCount / totalRequests,
      error_rate_percentage: (failureCount / totalRequests) * 100,
      status_codes: statusCodes,
      error_by_status_code: Object.fromEntries(
        Object.entries(statusCodes).filter(([code]) => parseInt(code) >= 400)
      ),
    };
  });
};

const mockLatencyMetrics = generateLatencyMetrics();
const mockFailureMetrics = generateFailureMetrics();

export const useMetricsStore = create<MetricsStore>((set, get) => ({
  latencyMetrics: mockLatencyMetrics,
  failureMetrics: mockFailureMetrics,
  selectedService: null,
  isLoading: false,
  lastUpdated: new Date(),

  setLatencyMetrics: (metrics) => set({ latencyMetrics: metrics, lastUpdated: new Date() }),

  setFailureMetrics: (metrics) => set({ failureMetrics: metrics, lastUpdated: new Date() }),

  setSelectedService: (service) => set({ selectedService: service }),

  setLoading: (loading) => set({ isLoading: loading }),

  refreshMetrics: async () => {
    set({ isLoading: true });
    
    // Simulate API call delay
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // Generate new mock data to simulate real-time updates
    const newLatencyMetrics = generateLatencyMetrics();
    const newFailureMetrics = generateFailureMetrics();
    
    set({
      latencyMetrics: newLatencyMetrics,
      failureMetrics: newFailureMetrics,
      isLoading: false,
      lastUpdated: new Date(),
    });
  },
}));

// Auto-refresh metrics every 30 seconds
setInterval(() => {
  const store = useMetricsStore.getState();
  if (!store.isLoading) {
    // Silently update metrics without showing loading state
    const newLatencyMetrics = generateLatencyMetrics();
    const newFailureMetrics = generateFailureMetrics();
    
    store.setLatencyMetrics(newLatencyMetrics);
    store.setFailureMetrics(newFailureMetrics);
  }
}, 30000);