/**
 * API Explorer Page
 *
 * In-app Scalar-based API explorer for hosted mock deployments.
 * Reached from the Hosted Mocks page when clicking "Open" on a deployment.
 */

import React, { useState, useEffect } from 'react';
import { Box, Typography, Chip, Button, Alert, CircularProgress } from '@mui/material';
import {
  ArrowBack as ArrowBackIcon,
  OpenInNew as OpenInNewIcon,
  ContentCopy as ContentCopyIcon,
} from '@mui/icons-material';
import { ApiReferenceReact as ApiReference } from '@scalar/api-reference-react';
import '@scalar/api-reference-react/style.css';
import {
  managementDocsUrl,
  managementHeaders,
  useManagementToken,
} from '@/hooks/useManagementToken';

interface DeploymentContext {
  id: string;
  name: string;
  deployment_url: string;
  status: string;
  openapi_spec_url?: string;
}

interface RouteInfo {
  method: string;
  path: string;
  operation_id?: string;
  summary?: string;
  description?: string;
}

interface ApiExplorerPageProps {
  deployment: DeploymentContext;
  onBack: () => void;
}

export const ApiExplorerPage: React.FC<ApiExplorerPageProps> = ({ deployment, onBack }) => {
  const [hasSpec, setHasSpec] = useState<boolean | null>(null);
  const [spec, setSpec] = useState<Record<string, unknown> | null>(null);
  const [routes, setRoutes] = useState<RouteInfo[]>([]);
  const [copied, setCopied] = useState(false);

  const isStoppedOrFailed = deployment.status === 'stopped' || deployment.status === 'failed';

  // `/__mockforge/api/spec` and `/__mockforge/routes` on the hosted mock
  // require the deployment's management token, reads included.
  const management = useManagementToken(deployment.id, !isStoppedOrFailed);
  const token = management.token;
  const tokenHeader = management.header;

  useEffect(() => {
    if (!token) return;
    let cancelled = false;
    const headers = managementHeaders({ token, header: tokenHeader });
    fetch(`${deployment.deployment_url}/__mockforge/api/spec`, {
      credentials: 'include',
      headers,
    })
      .then(async (r) => {
        if (r.ok) {
          const body: unknown = await r.json();
          if (typeof body === 'object' && body !== null) {
            if (!cancelled) {
              setSpec(body as Record<string, unknown>);
              setHasSpec(true);
            }
            return;
          }
        }
        if (cancelled) return;
        setHasSpec(false);
        // Fall back to loading routes
        return fetch(`${deployment.deployment_url}/__mockforge/routes`, {
          credentials: 'include',
          headers,
        })
          .then((rr) => rr.json())
          .then((data: { routes?: RouteInfo[] }) => {
            if (!cancelled) setRoutes(data.routes || []);
          })
          .catch(() => {});
      })
      .catch(() => {
        if (!cancelled) setHasSpec(false);
      });
    return () => {
      cancelled = true;
    };
  }, [token, tokenHeader, deployment.deployment_url]);

  const tokenUnavailable = !isStoppedOrFailed && management.status === 'unavailable';

  const handleCopyUrl = () => {
    navigator.clipboard.writeText(deployment.deployment_url).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 2,
          px: 3,
          py: 1.5,
          borderBottom: 1,
          borderColor: 'divider',
          flexShrink: 0,
        }}
      >
        <Button size="small" startIcon={<ArrowBackIcon />} onClick={onBack}>
          Back
        </Button>
        <Typography variant="h6" sx={{ fontWeight: 600 }}>
          {deployment.name}
        </Typography>
        <Chip
          label={deployment.status}
          color={
            deployment.status === 'active'
              ? 'success'
              : deployment.status === 'failed'
                ? 'error'
                : 'default'
          }
          size="small"
        />
        <Box sx={{ flexGrow: 1 }} />
        <Button
          size="small"
          startIcon={<ContentCopyIcon />}
          onClick={handleCopyUrl}
        >
          {copied ? 'Copied!' : 'Copy URL'}
        </Button>
        <Button
          size="small"
          startIcon={<OpenInNewIcon />}
          href={managementDocsUrl(deployment.deployment_url, token)}
          target="_blank"
          rel="noopener noreferrer"
        >
          Open in new tab
        </Button>
      </Box>

      {/* Content */}
      <Box sx={{ flexGrow: 1, overflow: 'auto' }}>
        {isStoppedOrFailed && (
          <Box sx={{ p: 4, textAlign: 'center' }}>
            <Alert severity={deployment.status === 'failed' ? 'error' : 'warning'} sx={{ mb: 2 }}>
              This deployment is {deployment.status}. The API explorer requires an active deployment.
            </Alert>
          </Box>
        )}

        {tokenUnavailable && (
          <Box sx={{ p: 4 }}>
            <Alert severity="warning">
              The API explorer is unavailable: reading this deployment&apos;s spec and routes
              requires its management token, which your account can&apos;t access.
            </Alert>
          </Box>
        )}

        {!isStoppedOrFailed && !tokenUnavailable && hasSpec === null && (
          <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: 300 }}>
            <CircularProgress />
          </Box>
        )}

        {!isStoppedOrFailed && !tokenUnavailable && hasSpec === true && spec && (
          <ApiReference
            configuration={{
              // Hand Scalar the spec we already fetched with the token so it
              // doesn't make its own unauthenticated request.
              content: spec,
              theme: 'default',
              hideDownloadButton: false,
              servers: [{ url: deployment.deployment_url, description: 'Mock deployment' }],
            }}
          />
        )}

        {!isStoppedOrFailed && !tokenUnavailable && hasSpec === false && (
          <Box sx={{ p: 4 }}>
            <Alert severity="info" sx={{ mb: 3 }}>
              No OpenAPI spec loaded for this deployment. Showing registered routes instead.
            </Alert>
            {routes.length > 0 ? (
              <Box component="table" sx={{ width: '100%', borderCollapse: 'collapse' }}>
                <thead>
                  <tr>
                    <Box component="th" sx={{ textAlign: 'left', p: 1, borderBottom: 1, borderColor: 'divider' }}>Method</Box>
                    <Box component="th" sx={{ textAlign: 'left', p: 1, borderBottom: 1, borderColor: 'divider' }}>Path</Box>
                    <Box component="th" sx={{ textAlign: 'left', p: 1, borderBottom: 1, borderColor: 'divider' }}>Summary</Box>
                  </tr>
                </thead>
                <tbody>
                  {routes.map((route, i) => (
                    <tr key={i}>
                      <Box component="td" sx={{ p: 1, borderBottom: 1, borderColor: 'divider' }}>
                        <Chip label={route.method} size="small" variant="outlined" />
                      </Box>
                      <Box component="td" sx={{ p: 1, borderBottom: 1, borderColor: 'divider', fontFamily: 'monospace' }}>
                        {route.path}
                      </Box>
                      <Box component="td" sx={{ p: 1, borderBottom: 1, borderColor: 'divider', color: 'text.secondary' }}>
                        {route.summary || route.description || '-'}
                      </Box>
                    </tr>
                  ))}
                </tbody>
              </Box>
            ) : (
              <Typography color="text.secondary">No routes registered.</Typography>
            )}
          </Box>
        )}
      </Box>
    </Box>
  );
};

export default ApiExplorerPage;
