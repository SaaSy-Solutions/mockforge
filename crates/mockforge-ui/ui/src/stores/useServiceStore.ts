import { logger } from '@/utils/logger';
import { create } from 'zustand';
import type { ServiceInfo, RouteInfo } from '../types';

interface ServiceStore {
  services: ServiceInfo[];
  filteredRoutes: RouteInfo[];
  setServices: (services: ServiceInfo[]) => void;
  updateService: (serviceId: string, updates: Partial<ServiceInfo>) => void;
  toggleRoute: (serviceId: string, routeId: string, enabled: boolean) => void;
  addService: (service: ServiceInfo) => void;
  removeService: (serviceId: string) => void;
  setGlobalSearch: (query: string | undefined) => void;
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

  setServices: (services) => set({ services, filteredRoutes: filterRoutes(services) }),

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
