/**
 * Hosted mock management token.
 *
 * Every MockForge control-plane route on a hosted mock's data port
 * (`/__mockforge/*` except the static docs page, chaos, recorder, ...)
 * requires the deployment's management token, for reads as well as writes.
 * The registry hands it out at
 * `GET /api/v1/hosted-mocks/{id}/management-token` to callers holding the
 * HostedMockUpdate permission.
 */

import { useEffect, useState } from 'react';
import { fetchJsonWithErrorBody } from '@/services/api/client';

export const MANAGEMENT_TOKEN_HEADER = 'X-MockForge-Management-Token';

/** WebSocket subprotocol the server selects for management streams. */
export const MANAGEMENT_WS_PROTOCOL = 'mockforge-management';

export interface ManagementToken {
  token: string;
  header: string;
}

export type ManagementTokenStatus = 'idle' | 'loading' | 'ready' | 'unavailable';

export interface UseManagementTokenReturn {
  token: string | null;
  header: string;
  status: ManagementTokenStatus;
  error: string | null;
}

export async function fetchManagementToken(deploymentId: string): Promise<ManagementToken> {
  const body = await fetchJsonWithErrorBody(
    `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/management-token`,
  );
  if (
    typeof body !== 'object' ||
    body === null ||
    typeof (body as { token?: unknown }).token !== 'string'
  ) {
    throw new Error('Malformed management token response');
  }
  const { token, header } = body as { token: string; header?: unknown };
  return { token, header: typeof header === 'string' ? header : MANAGEMENT_TOKEN_HEADER };
}

/** Headers to send on a control-plane request to a hosted mock. */
export function managementHeaders(token: ManagementToken | string): Record<string, string> {
  return typeof token === 'string'
    ? { [MANAGEMENT_TOKEN_HEADER]: token }
    : { [token.header || MANAGEMENT_TOKEN_HEADER]: token.token };
}

/** WebSocket subprotocols carrying the token (browsers can't set WS headers). */
export function managementWsProtocols(token: string): string[] {
  return [MANAGEMENT_WS_PROTOCOL, `mockforge-token.${token}`];
}

/**
 * Link to a hosted mock's docs page. The token rides in the URL fragment,
 * which browsers never send to the server; the page reads it client-side.
 */
export function managementDocsUrl(deploymentUrl: string, token?: string | null): string {
  const base = `${deploymentUrl.replace(/\/+$/, '')}/__mockforge/docs`;
  return token ? `${base}#management_token=${encodeURIComponent(token)}` : base;
}

/**
 * Fetch the management token for a hosted deployment once per id. Failures
 * (403 without HostedMockUpdate, 404, network) settle on `unavailable`
 * without retrying, so callers can degrade instead of looping.
 */
export function useManagementToken(
  deploymentId: string | null | undefined,
  enabled = true,
): UseManagementTokenReturn {
  const [state, setState] = useState<{
    id: string | null;
    value: ManagementToken | null;
    error: string | null;
  }>({ id: null, value: null, error: null });

  const active = enabled && !!deploymentId;

  useEffect(() => {
    if (!active || !deploymentId) return;
    let cancelled = false;
    fetchManagementToken(deploymentId)
      .then((value) => {
        if (!cancelled) setState({ id: deploymentId, value, error: null });
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setState({
            id: deploymentId,
            value: null,
            error: err instanceof Error ? err.message : 'Could not load the management token',
          });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [active, deploymentId]);

  if (!active) {
    return { token: null, header: MANAGEMENT_TOKEN_HEADER, status: 'idle', error: null };
  }
  if (state.id !== deploymentId) {
    return { token: null, header: MANAGEMENT_TOKEN_HEADER, status: 'loading', error: null };
  }
  if (state.value) {
    return {
      token: state.value.token,
      header: state.value.header,
      status: 'ready',
      error: null,
    };
  }
  return {
    token: null,
    header: MANAGEMENT_TOKEN_HEADER,
    status: 'unavailable',
    error: state.error,
  };
}
