import { logger } from '@/utils/logger';
import { create } from 'zustand';
import type { ServiceInfo, RouteInfo } from '../types';
import { authenticatedFetch } from '../utils/apiClient';
import { cloudServicesApi } from '../services/api';
import type {
  CloudService,
  CloudServiceCreatePayload,
  CloudServiceUpdatePayload,
} from '../services/api/cloudServices';

const isCloud = !!import.meta.env.VITE_API_BASE_URL;

interface ServiceStore {
  services: ServiceInfo[];
  filteredRoutes: RouteInfo[];
  isLoading: boolean;
  error: string | null;
  isCloud: boolean;
  mutationError: string | null;
  workspaceFilter: string | null;
  setServices: (services: ServiceInfo[]) => void;
  updateService: (serviceId: string, updates: Partial<ServiceInfo>) => Promise<void>;
  updateServiceDetails: (
    serviceId: string,
    details: { name?: string; description?: string; base_url?: string; tags?: string[]; workspace_id?: string | null }
  ) => Promise<void>;
  toggleRoute: (serviceId: string, routeId: string, enabled: boolean) => Promise<void>;
  addService: (service: ServiceInfo) => void;
  createService: (payload: CloudServiceCreatePayload) => Promise<ServiceInfo>;
  removeService: (serviceId: string) => Promise<void>;
  setGlobalSearch: (query: string | undefined) => void;
  setWorkspaceFilter: (workspaceId: string | null) => Promise<void>;
  fetchServices: (options?: { workspaceId?: string | null }) => Promise<void>;
  clearError: () => void;
  clearMutationError: () => void;
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

const SHOULD_USE_MOCK_FALLBACK =
  !isCloud && (import.meta.env.DEV || import.meta.env.VITE_ENABLE_MOCK_DATA === 'true');

const routeKey = (route: Pick<RouteInfo, 'method' | 'path'>): string =>
  route.method ? `${route.method}-${route.path}` : route.path;

const coerceRoutes = (value: unknown): RouteInfo[] => {
  if (!Array.isArray(value)) return [];
  return value
    .map((raw) => {
      if (!raw || typeof raw !== 'object') return null;
      const r = raw as Record<string, unknown>;
      const path = typeof r.path === 'string' ? r.path : '';
      if (!path) return null;
      const method = typeof r.method === 'string' ? r.method : 'ANY';
      return {
        id: typeof r.id === 'string' ? r.id : `${method}-${path}`,
        method,
        path,
        statusCode: typeof r.status_code === 'number' ? r.status_code : 200,
        priority: typeof r.priority === 'number' ? r.priority : 1,
        has_fixtures: typeof r.has_fixtures === 'boolean' ? r.has_fixtures : false,
        request_count: typeof r.request_count === 'number' ? r.request_count : 0,
        error_count: typeof r.error_count === 'number' ? r.error_count : 0,
        latency_ms: typeof r.latency_ms === 'number' ? r.latency_ms : 0,
        enabled: r.enabled !== false,
        tags: Array.isArray(r.tags) ? (r.tags.filter((t) => typeof t === 'string') as string[]) : [],
      } as RouteInfo;
    })
    .filter((r): r is RouteInfo => r !== null);
};

const coerceTags = (value: unknown): string[] => {
  if (!Array.isArray(value)) return [];
  return value.filter((t): t is string => typeof t === 'string');
};

const mapCloudService = (svc: CloudService): ServiceInfo => {
  const routes = coerceRoutes(svc.routes).map((r) => ({ ...r, service_id: svc.id }));
  return {
    id: svc.id,
    name: svc.name,
    description: svc.description || undefined,
    baseUrl: svc.base_url || '',
    enabled: svc.enabled,
    tags: coerceTags(svc.tags),
    routes,
    createdAt: svc.created_at,
    updatedAt: svc.updated_at,
    workspace_id: svc.workspace_id ?? null,
  };
};

export const useServiceStore = create<ServiceStore>((set, get) => ({
  services: SHOULD_USE_MOCK_FALLBACK ? mockServices : [],
  filteredRoutes: SHOULD_USE_MOCK_FALLBACK ? filterRoutes(mockServices) : [],
  isLoading: false,
  error: null,
  isCloud,
  mutationError: null,
  workspaceFilter: null,

  setServices: (services) => set({ services, filteredRoutes: filterRoutes(services) }),

  setWorkspaceFilter: async (workspaceId) => {
    set({ workspaceFilter: workspaceId });
    await get().fetchServices({ workspaceId });
  },

  fetchServices: async (options) => {
    set({ isLoading: true, error: null });
    try {
      if (isCloud) {
        const workspaceId = options?.workspaceId ?? get().workspaceFilter ?? undefined;
        const cloudServices = await cloudServicesApi.list(
          workspaceId ? { workspaceId } : undefined
        );
        const services = cloudServices.map(mapCloudService);
        set({ services, filteredRoutes: filterRoutes(services), isLoading: false });
        return;
      }

      const response = await authenticatedFetch('/__mockforge/routes');
      if (!response.ok) {
        throw new Error(`Failed to fetch routes: ${response.statusText}`);
      }
      const json = await response.json();
      const routes = Array.isArray(json) ? json : (json.data || []);

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

      // Fall back to mock data only in development-mode workflows.
      if (services.length === 0 && SHOULD_USE_MOCK_FALLBACK) {
        set({ services: mockServices, filteredRoutes: filterRoutes(mockServices), isLoading: false });
      } else {
        set({ services, filteredRoutes: filterRoutes(services), isLoading: false });
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to fetch services';
      logger.error('Failed to fetch services', error);
      if (SHOULD_USE_MOCK_FALLBACK) {
        set({
          services: mockServices,
          filteredRoutes: filterRoutes(mockServices),
          error: errorMessage,
          isLoading: false,
        });
      } else {
        set({ services: [], filteredRoutes: [], error: errorMessage, isLoading: false });
      }
    }
  },

  clearError: () => set({ error: null }),
  clearMutationError: () => set({ mutationError: null }),

  updateService: async (serviceId, updates) => {
    // Optimistic local update first
    set((state) => ({
      services: state.services.map((service) =>
        service.id === serviceId ? { ...service, ...updates } : service
      ),
    }));

    if (!isCloud) return;

    try {
      const patch: CloudServiceUpdatePayload = {};
      if (typeof updates.enabled === 'boolean') patch.enabled = updates.enabled;
      if (typeof updates.name === 'string') patch.name = updates.name;
      if (typeof updates.description === 'string') patch.description = updates.description;
      if (typeof updates.baseUrl === 'string') patch.base_url = updates.baseUrl;
      if (Array.isArray(updates.tags)) patch.tags = updates.tags;
      if (Object.keys(patch).length === 0) return;

      const updated = await cloudServicesApi.update(serviceId, patch);
      const mapped = mapCloudService(updated);
      set((state) => ({
        services: state.services.map((service) =>
          service.id === serviceId ? mapped : service
        ),
        filteredRoutes: filterRoutes(
          state.services.map((service) => (service.id === serviceId ? mapped : service))
        ),
      }));
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Failed to update service';
      logger.error('Failed to persist service update', error);
      set({ mutationError: msg });
      // Re-fetch to restore server-authoritative state after a failed optimistic write
      void get().fetchServices();
    }
  },

  updateServiceDetails: async (serviceId, details) => {
    if (!isCloud) {
      set({ mutationError: 'Editing service details requires cloud mode.' });
      return;
    }
    const patch: CloudServiceUpdatePayload = {};
    if (typeof details.name === 'string') patch.name = details.name;
    if (typeof details.description === 'string') patch.description = details.description;
    if (typeof details.base_url === 'string') patch.base_url = details.base_url;
    if (Array.isArray(details.tags)) patch.tags = details.tags;
    if (details.workspace_id !== undefined) patch.workspace_id = details.workspace_id;
    if (Object.keys(patch).length === 0) return;

    try {
      const updated = await cloudServicesApi.update(serviceId, patch);
      const mapped = mapCloudService(updated);
      set((state) => {
        const services = state.services.map((service) =>
          service.id === serviceId ? mapped : service
        );
        return { services, filteredRoutes: filterRoutes(services) };
      });
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Failed to update service';
      logger.error('Failed to update service details', error);
      set({ mutationError: msg });
      throw error;
    }
  },

  toggleRoute: async (serviceId, routeId, enabled) => {
    const previous = get().services;
    const nextServices = previous.map((service) =>
      service.id === serviceId
        ? {
            ...service,
            routes: service.routes.map((route) => {
              const id = routeKey(route);
              return id === routeId ? { ...route, enabled } : route;
            }),
          }
        : service
    );
    set({ services: nextServices, filteredRoutes: filterRoutes(nextServices) });

    if (!isCloud) return;

    const target = nextServices.find((s) => s.id === serviceId);
    if (!target) return;

    try {
      // Persist the full routes array — CloudService.routes is stored as JSON.
      const routesPayload = target.routes.map((r) => ({
        id: r.id,
        method: r.method,
        path: r.path,
        enabled: r.enabled !== false,
        status_code: r.statusCode,
        priority: r.priority,
        has_fixtures: r.has_fixtures,
        tags: r.tags ?? [],
      }));
      const updated = await cloudServicesApi.update(serviceId, { routes: routesPayload });
      const mapped = mapCloudService(updated);
      set((state) => {
        const services = state.services.map((service) =>
          service.id === serviceId ? mapped : service
        );
        return { services, filteredRoutes: filterRoutes(services) };
      });
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Failed to update route';
      logger.error('Failed to persist route toggle', error);
      set({ mutationError: msg, services: previous, filteredRoutes: filterRoutes(previous) });
    }
  },

  addService: (service) => set((state) => {
    const services = [...state.services, service];
    return { services, filteredRoutes: filterRoutes(services) };
  }),

  createService: async (payload) => {
    if (!isCloud) {
      throw new Error('Creating services requires cloud mode.');
    }
    try {
      const created = await cloudServicesApi.create(payload);
      const mapped = mapCloudService(created);
      set((state) => {
        const services = [...state.services, mapped];
        return { services, filteredRoutes: filterRoutes(services) };
      });
      return mapped;
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Failed to create service';
      logger.error('Failed to create service', error);
      set({ mutationError: msg });
      throw error;
    }
  },

  removeService: async (serviceId) => {
    const previous = get().services;
    // Optimistic removal
    set((state) => {
      const services = state.services.filter((service) => service.id !== serviceId);
      return { services, filteredRoutes: filterRoutes(services) };
    });

    if (!isCloud) return;

    try {
      await cloudServicesApi.remove(serviceId);
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Failed to delete service';
      logger.error('Failed to delete service', error);
      set({ mutationError: msg, services: previous, filteredRoutes: filterRoutes(previous) });
      throw error;
    }
  },

  setGlobalSearch: (query) => set((state) => ({
    filteredRoutes: filterRoutes(state.services, query),
  })),
}));
