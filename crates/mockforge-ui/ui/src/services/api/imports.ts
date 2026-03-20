/**
 * Import API service — Postman, Insomnia, cURL, and OpenAPI import operations.
 */
import type {
  ImportRequest,
  ImportResponse,
  ImportHistoryResponse,
} from '../../types';
import { fetchJson } from './client';

class ImportApiService {
  async importPostman(request: ImportRequest): Promise<ImportResponse> {
    return fetchJson('/__mockforge/import/postman', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async importInsomnia(request: ImportRequest): Promise<ImportResponse> {
    return fetchJson('/__mockforge/import/insomnia', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async importCurl(request: ImportRequest): Promise<ImportResponse> {
    return fetchJson('/__mockforge/import/curl', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async importOpenApi(request: ImportRequest): Promise<ImportResponse> {
    return fetchJson('/__mockforge/import/openapi', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async previewImport(request: ImportRequest): Promise<ImportResponse> {
    return fetchJson('/__mockforge/import/preview', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async getImportHistory(): Promise<ImportHistoryResponse> {
    return fetchJson('/__mockforge/import/history') as Promise<ImportHistoryResponse>;
  }

  async clearImportHistory(): Promise<void> {
    return fetchJson('/__mockforge/import/history/clear', {
      method: 'POST',
    }) as Promise<void>;
  }
}

export { ImportApiService };
