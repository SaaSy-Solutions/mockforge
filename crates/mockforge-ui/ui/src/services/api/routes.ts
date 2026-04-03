/**
 * Routes API service — route listing.
 */
import type { RouteInfo } from '../../types';
import { fetchJson } from './client';

const isCloud = !!import.meta.env.VITE_API_BASE_URL;
const ROUTES_API_BASE = isCloud ? '/api/v1/services' : '/__mockforge/routes';

class RoutesApiService {
  async getRoutes(): Promise<RouteInfo[]> {
    if (isCloud) {
      // In cloud mode, services contain routes as a nested field
      // Return an empty array — route listing is handled by the services page
      return [];
    }
    return fetchJson(ROUTES_API_BASE) as Promise<RouteInfo[]>;
  }
}

export { RoutesApiService };
