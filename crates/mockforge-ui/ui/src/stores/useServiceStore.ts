import { logger } from '@/utils/logger';
import { create } from 'zustand';
import type { ServiceInfo, RouteInfo } from '../types';

interface ServiceStore {
  services: ServiceInfo[];
  filteredRoutes: RouteInfo[];
  isLoading: boolean;
  error: string | null;
  setServices: (services: ServiceInfo[]) => void;
  updateService: (serviceId: string, updates: Partial<ServiceInfo>) => void;
  toggleRoute: (serviceId: string, routeId: string, enabled: boolean) => void;
  addService: (service: ServiceInfo) => void;
  removeService: (serviceId: string) => void;
  setGlobalSearch: (query: string | undefined) => void;
  fetchServices: () => Promise<void>;
  clearError: () => void;
}

// Mock data for development
const mockServices: ServiceInfo[] = [
  {
    id: 'user-service',
    name: 'User Service',
    baseUrl: 'http://localhost:3000',
    enabled: true,
    tags: ['api', 'users'],
    description: 'Handles user authentication and profile management',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    routes: [
      {
        id: 'user-service-get-users',
        method: 'GET',
        path: '/api/users',
        statusCode: 200,
        priority: 1,
        has_fixtures: true,
        request_count: 234,
        error_count: 2,
        latency_ms: 45,
        enabled: true,
        service_id: 'user-service',
        tags: ['api', 'users'],
      },
      {
        id: 'user-service-post-users',
        method: 'POST',
        path: '/api/users',
        statusCode: 201,
        priority: 1,
        has_fixtures: true,
        request_count: 89,
        error_count: 0,
        latency_ms: 67,
        enabled: true,
        service_id: 'user-service',
        tags: ['api', 'users'],
      },
      {
        id: 'user-service-get-user-id',
        method: 'GET',
        path: '/api/users/{id}',
        statusCode: 200,
        priority: 1,
        has_fixtures: true,
        request_count: 156,
        error_count: 1,
        latency_ms: 32,
        enabled: false,
        service_id: 'user-service',
        tags: ['api', 'users'],
      },
    ],
  },
  {
    id: 'order-service',
    name: 'Order Service',
    baseUrl: 'http://localhost:3001',
    enabled: true,
    tags: ['api', 'orders', 'ecommerce'],
    description: 'Manages orders and order processing',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    routes: [
      {
        id: 'order-service-get-orders',
        method: 'GET',
        path: '/api/orders',
        statusCode: 200,
        priority: 1,
        has_fixtures: true,
        request_count: 445,
        error_count: 5,
        latency_ms: 78,
        enabled: true,
        service_id: 'order-service',
        tags: ['api', 'orders'],
      },
      {
        id: 'order-service-post-orders',
        method: 'POST',
        path: '/api/orders',
        statusCode: 201,
        priority: 1,
        has_fixtures: true,
        request_count: 123,
        error_count: 3,
        latency_ms: 234,
        enabled: true,
        service_id: 'order-service',
        tags: ['api', 'orders'],
      },
    ],
  },
  {
    id: 'grpc-inventory',
    name: 'Inventory gRPC',
    baseUrl: 'grpc://localhost:50051',
    enabled: false,
    tags: ['grpc', 'inventory'],
    description: 'gRPC service for inventory management',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    routes: [
      {
        id: 'grpc-inventory-get-item',
        method: 'GRPC',
        path: 'inventory.InventoryService/GetItem',
        statusCode: 0,
        priority: 1,
        has_fixtures: false,
        request_count: 67,
        error_count: 0,
        latency_ms: 23,
        enabled: false,
        service_id: 'grpc-inventory',
        tags: ['grpc', 'inventory'],
      },
      {
        id: 'grpc-inventory-update-stock',
        method: 'GRPC',
        path: 'inventory.InventoryService/UpdateStock',
        statusCode: 0,
        priority: 1,
        has_fixtures: false,
        request_count: 34,
        error_count: 1,
        latency_ms: 56,
        enabled: false,
        service_id: 'grpc-inventory',
        tags: ['grpc', 'inventory'],
      },
    ],
  },
];

const filterRoutes = (services: ServiceInfo[], query?: string): RouteInfo[] => {
  const allRoutes = services.flatMap(s => s.routes.map(r => ({ ...r })));
  if (!query) return allRoutes;
  const q = query.toLowerCase();
  return allRoutes.filter(r =>
    (r.method ? r.method.toLowerCase().includes(q) : false) ||
    r.path.toLowerCase().includes(q) ||
    (r.tags && r.tags.some(t => t.toLowerCase().includes(q)))
  );
};

export const useServiceStore = create<ServiceStore>((set, _get) => ({
  services: mockServices,
  filteredRoutes: filterRoutes(mockServices),
  isLoading: false,
  error: null,

  setServices: (services) => set({ services, filteredRoutes: filterRoutes(services) }),

  fetchServices: async () => {
    set({ isLoading: true, error: null });
    try {
      const response = await fetch('/__mockforge/routes');
      if (!response.ok) {
        throw new Error(`Failed to fetch routes: ${response.statusText}`);
      }
      const routes = await response.json();

      // Transform routes into services grouped by base path or tag
      const serviceMap = new Map<string, ServiceInfo>();

      for (const route of routes) {
        // Extract service name from path (e.g., /api/users -> users-service)
        const pathParts = (route.path || '').split('/').filter(Boolean);
        const serviceName = pathParts[1] || pathParts[0] || 'default';
        const serviceId = `${serviceName}-service`;

        if (!serviceMap.has(serviceId)) {
          serviceMap.set(serviceId, {
            id: serviceId,
            name: serviceName.charAt(0).toUpperCase() + serviceName.slice(1) + ' Service',
            baseUrl: window.location.origin,
            enabled: true,
            tags: [],
            description: `Routes for ${serviceName}`,
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            routes: [],
          });
        }

        const service = serviceMap.get(serviceId)!;
        service.routes.push({
          id: `${serviceId}-${route.method}-${route.path}`.replace(/[^a-zA-Z0-9-]/g, '-'),
          method: route.method || 'ANY',
          path: route.path || '/',
          statusCode: route.status_code || 200,
          priority: route.priority || 1,
          has_fixtures: route.has_fixtures || false,
          request_count: route.request_count || 0,
          error_count: route.error_count || 0,
          latency_ms: route.latency_ms || 0,
          enabled: route.enabled !== false,
          service_id: serviceId,
          tags: route.tags || [],
        });
      }

      const services = Array.from(serviceMap.values());

      // Fall back to mock data if no routes returned
      if (services.length === 0) {
        set({ services: mockServices, filteredRoutes: filterRoutes(mockServices), isLoading: false });
      } else {
        set({ services, filteredRoutes: filterRoutes(services), isLoading: false });
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to fetch services';
      logger.error('Failed to fetch services', error);
      // Fall back to mock data on error for development
      set({ services: mockServices, filteredRoutes: filterRoutes(mockServices), error: errorMessage, isLoading: false });
    }
  },

  clearError: () => set({ error: null }),

  updateService: (serviceId, updates) => set((state) => ({
    services: state.services.map(service =>
      service.id === serviceId
        ? { ...service, ...updates }
        : service
    ),
  })),

  toggleRoute: (serviceId, routeId, enabled) => set((state) => ({
    services: state.services.map(service =>
      service.id === serviceId
        ? {
            ...service,
            routes: service.routes.map(route => {
              const id = route.method ? `${route.method}-${route.path}` : route.path;
              return id === routeId ? { ...route, enabled } : route;
            }),
          }
        : service
    ),
  })),

  addService: (service) => set((state) => ({
    services: [...state.services, service],
    filteredRoutes: filterRoutes([...state.services, service]),
  })),

  removeService: (serviceId) => set((state) => ({
    services: state.services.filter(service => service.id !== serviceId),
    filteredRoutes: filterRoutes(state.services.filter(service => service.id !== serviceId)),
  })),

  setGlobalSearch: (query) => set((state) => ({
    filteredRoutes: filterRoutes(state.services, query),
  })),
}));
