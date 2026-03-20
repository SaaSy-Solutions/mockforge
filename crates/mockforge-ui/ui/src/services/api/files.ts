/**
 * Files API service — file content retrieval and saving.
 */
import type { FileContentRequest, FileContentResponse, SaveFileRequest } from '../../types';
import { fetchJson } from './client';

class FilesApiService {
  async getFileContent(request: FileContentRequest): Promise<FileContentResponse> {
    return fetchJson('/__mockforge/files/content', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<FileContentResponse>;
  }

  async saveFileContent(request: SaveFileRequest): Promise<{ message: string }> {
    return fetchJson('/__mockforge/files/save', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ message: string }>;
  }
}

export { FilesApiService };
