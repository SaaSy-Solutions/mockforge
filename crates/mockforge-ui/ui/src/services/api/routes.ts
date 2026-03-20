/**
 * Routes API service — route listing.
 */
import type { RouteInfo } from '../../types';
import { fetchJson } from './client';

class RoutesApiService {
  async getRoutes(): Promise<RouteInfo[]> {
    return fetchJson('/__mockforge/routes') as Promise<RouteInfo[]>;
  }
}

export { RoutesApiService };
