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

// Make this file a module so the `declare global` augmentation above applies.
export {};
