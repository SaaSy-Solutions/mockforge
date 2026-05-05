/**
 * Cloud Runs control-plane API — DX helpers that aren't tied to one
 * specific run kind.
 *
 * Right now: just the data-driven Tigris upload URL endpoint. Per-kind
 * trigger flows continue to live on cloudTestRunsApi.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export interface DataDrivenUploadUrlRequest {
  /** File extension hint (`csv`, `json`). Defaults to `csv` server-side. */
  extension?: string;
  /** PUT URL lifetime in seconds. Defaults to 300, capped at 3600. */
  upload_ttl_seconds?: number;
  /** GET URL lifetime in seconds. Defaults to 86400, capped at 604800. */
  data_ttl_seconds?: number;
}

export interface DataDrivenUploadUrlResponse {
  /** Presigned PUT URL — upload the CSV/JSON directly here. Short-lived. */
  upload_url: string;
  /** Presigned GET URL — paste into the suite config as `data_url`. Longer-lived. */
  data_url: string;
  /** Tigris object key (for reference / manual cleanup). */
  object_key: string;
  upload_expires_in_seconds: number;
  data_expires_in_seconds: number;
}

class CloudRunsApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud Runs ${method} only works in cloud mode.`);
    }
  }

  /**
   * Request a presigned PUT/GET URL pair for a data-driven test-vector
   * upload. UI uploads directly to the returned `upload_url` (skipping
   * the registry as a relay), then puts the `data_url` in the suite
   * config so the runner fetches it at run time.
   */
  async requestDataDrivenUploadUrl(
    body: DataDrivenUploadUrlRequest = {},
  ): Promise<DataDrivenUploadUrlResponse> {
    this.guard('requestDataDrivenUploadUrl');
    return fetchJsonWithErrorBody('/api/v1/cloud-runs/data-driven/upload-url', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<DataDrivenUploadUrlResponse>;
  }
}

export const cloudRunsApi = new CloudRunsApi();
