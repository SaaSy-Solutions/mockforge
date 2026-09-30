import { useState } from 'react';
import { Box, Button, IconButton, Tooltip, Typography } from '@mui/material';
import { ContentCopy as CopyIcon, Key as KeyIcon } from '@mui/icons-material';
import { fetchJsonWithErrorBody } from '@/services/api/client';

interface ManagementToken {
  token: string;
  header: string;
}

/**
 * Reveals a hosted mock's management token. Control-plane writes on the mock
 * (creating mocks, chaos, time travel, recorder) need it in the
 * X-MockForge-Management-Token header; reads and the mocked API itself do not.
 */
export function ManagementTokenField({ deploymentId }: { deploymentId: string }) {
  const [value, setValue] = useState<ManagementToken | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const reveal = async () => {
    setError(null);
    try {
      setValue(
        (await fetchJsonWithErrorBody(
          `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/management-token`,
        )) as ManagementToken,
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not load the token');
    }
  };

  const copy = async () => {
    if (!value) return;
    try {
      await navigator.clipboard.writeText(value.token);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      setError('Copy failed; select the token and copy it manually.');
    }
  };

  return (
    <Box>
      <Typography variant="caption" color="text.secondary">
        Management token
      </Typography>
      {value ? (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <Typography variant="body2" sx={{ fontFamily: 'monospace', wordBreak: 'break-all' }}>
            {value.token}
          </Typography>
          <Tooltip title={copied ? 'Copied' : 'Copy'}>
            <IconButton size="small" aria-label="Copy management token" onClick={copy}>
              <CopyIcon fontSize="small" />
            </IconButton>
          </Tooltip>
        </Box>
      ) : (
        <Box>
          <Button size="small" startIcon={<KeyIcon />} onClick={reveal}>
            Reveal
          </Button>
        </Box>
      )}
      <Typography variant="caption" color="text.secondary" component="p">
        {value
          ? `Send as ${value.header} on writes to /__mockforge/* and other MockForge control routes. Reads and your mocked API don't need it.`
          : 'Needed to change this mock through its management API from curl or your own client.'}
      </Typography>
      {error && (
        <Typography variant="caption" color="error">
          {error}
        </Typography>
      )}
    </Box>
  );
}
