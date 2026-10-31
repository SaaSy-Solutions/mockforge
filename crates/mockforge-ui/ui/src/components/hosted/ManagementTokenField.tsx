import { useState } from 'react';
import { Copy, KeyRound } from 'lucide-react';
import { Button } from '../ui/button';
import { fetchManagementToken, type ManagementToken } from '@/hooks/useManagementToken';

/**
 * Reveals a hosted mock's management token. Every control-plane route on the
 * mock (`/__mockforge/*` except the docs page, chaos, time travel, recorder),
 * reads as well as writes, needs it in the X-MockForge-Management-Token
 * header; only the mocked API itself does not.
 */
export function ManagementTokenField({ deploymentId }: { deploymentId: string }) {
  const [value, setValue] = useState<ManagementToken | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const reveal = async () => {
    setError(null);
    try {
      setValue(await fetchManagementToken(deploymentId));
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
    <div className="space-y-1">
      <p className="text-xs text-muted-foreground">Management token</p>
      {value ? (
        <div className="flex min-w-0 items-start gap-2">
          <code className="min-w-0 break-all text-sm">{value.token}</code>
          <Button type="button" variant="ghost" size="icon" className="h-8 w-8 shrink-0"
            title={copied ? 'Copied' : 'Copy'} aria-label="Copy management token" onClick={copy}>
            <Copy className="h-4 w-4" aria-hidden="true" />
          </Button>
          <span role="status" className="sr-only">{copied ? 'Management token copied' : ''}</span>
        </div>
      ) : (
        <Button type="button" variant="ghost" size="sm" className="gap-2" onClick={reveal}>
          <KeyRound className="h-4 w-4" aria-hidden="true" />Reveal
        </Button>
      )}
      <p className="text-xs text-muted-foreground">
        {value
          ? `Send as ${value.header} on every request to /__mockforge/* and other MockForge control routes, reads included. Your mocked API doesn't need it.`
          : 'Needed to read or change this mock through its management API from curl or your own client.'}
      </p>
      {error && <p role="alert" className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
