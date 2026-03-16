import { logger } from '@/utils/logger';
/// <reference types="vite/client" />

interface ExplorerDeploymentContext {
  id: string;
  name: string;
  deployment_url: string;
  status: string;
  openapi_spec_url?: string;
}

declare global {
  interface Window {
    __mockforge_explorer_deployment?: ExplorerDeploymentContext;
  }
}
