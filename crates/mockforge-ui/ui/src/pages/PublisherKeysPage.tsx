// Publisher SBOM attestation keys — settings page.
//
// A publisher registers one or more Ed25519 public keys on this page.
// Their matching private key stays on their own machine; signing the
// SBOM happens locally via `mockforge-plugin key sign` (or automatically
// with `publish --sign`), and the server verifies the signature against
// any of the registered public keys at publish time. Revoked keys are
// soft-deleted — still visible in history, but the verifier skips them.
//
// We deliberately do not offer key *generation* in the browser. The
// private key should never round-trip through a network service, and
// the UI isn't a safe place to hold it.

import React, { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Button,
  Card,
  CardContent,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  IconButton,
  LinearProgress,
  Paper,
  Stack,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material';
import {
  Add as AddIcon,
  Delete as DeleteIcon,
  Key as KeyIcon,
  ContentCopy as CopyIcon,
  VerifiedUser as VerifiedIcon,
} from '@mui/icons-material';
import { authenticatedFetch } from '../utils/apiClient';

interface PublicKey {
  id: string;
  algorithm: string;
  publicKeyB64: string;
  label: string;
  createdAt: string;
  revokedAt?: string | null;
}

interface ListResponse {
  keys: PublicKey[];
}

async function fetchKeys(): Promise<PublicKey[]> {
  const resp = await authenticatedFetch('/api/v1/users/me/public-keys');
  if (!resp.ok) {
    const body = await resp.text().catch(() => '');
    throw new Error(`Failed to load keys (${resp.status}): ${body}`);
  }
  const data: ListResponse = await resp.json();
  return data.keys;
}

async function addKey(req: {
  algorithm: string;
  publicKeyB64: string;
  label: string;
}): Promise<PublicKey> {
  const resp = await authenticatedFetch('/api/v1/users/me/public-keys', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!resp.ok) {
    const body = await resp.text().catch(() => '');
    throw new Error(`Failed to register key (${resp.status}): ${body}`);
  }
  return resp.json();
}

async function revokeKey(id: string): Promise<void> {
  const resp = await authenticatedFetch(`/api/v1/users/me/public-keys/${id}`, {
    method: 'DELETE',
  });
  if (!resp.ok) {
    const body = await resp.text().catch(() => '');
    throw new Error(`Failed to revoke key (${resp.status}): ${body}`);
  }
}

/// Short fingerprint for UI display — first 16 hex chars of SHA-256 of
/// the decoded key. Lets a human eyeball-verify that the key on screen
/// matches the one they just generated without needing to compare the
/// full base64 string.
///
/// Uses `crypto.subtle.digest` when available (HTTPS, modern browsers)
/// and falls back to a compact pure-JS SHA-256 implementation in dev
/// setups served over `http://localhost` where `crypto.subtle` is
/// missing. The output must match what the CLI prints
/// (`mockforge-plugin key list`) so the two are comparable.
async function fingerprint(publicKeyB64: string): Promise<string> {
  try {
    const raw = Uint8Array.from(atob(normalizeBase64(publicKeyB64)), (c) =>
      c.charCodeAt(0)
    );
    const hash = await sha256(raw);
    const bytes = Array.from(hash.slice(0, 8));
    return bytes.map((b) => b.toString(16).padStart(2, '0')).join('');
  } catch {
    return '—';
  }
}

function normalizeBase64(s: string): string {
  // The server normalizes URL-safe base64 to standard; do the same
  // client-side so pasted url-safe keys still fingerprint cleanly.
  const stdish = s.replace(/-/g, '+').replace(/_/g, '/');
  const pad = stdish.length % 4;
  return pad ? stdish + '='.repeat(4 - pad) : stdish;
}

/// Prefer `crypto.subtle` (native, fast, timing-safe). Fall back to a
/// pure-JS SHA-256 when `subtle` is unavailable — which happens in
/// two real scenarios:
///   1. Dev server on `http://localhost:<port>` that isn't a "secure
///      context" (some browser/OS combos don't localhost-allowlist).
///   2. Users behind corporate CSPs that block `subtle` at import time.
/// The fallback is not timing-safe, but fingerprinting a *public* key
/// has no secret input to protect, so the tradeoff is fine.
async function sha256(bytes: Uint8Array): Promise<Uint8Array> {
  if (typeof crypto !== 'undefined' && crypto.subtle && crypto.subtle.digest) {
    try {
      const buf = await crypto.subtle.digest('SHA-256', bytes);
      return new Uint8Array(buf);
    } catch {
      // fall through to pure-JS
    }
  }
  return sha256Fallback(bytes);
}

/// Compact SHA-256 implementation for non-secure-context dev setups.
/// Lifted from the FIPS 180-4 pseudocode; correctness is spot-checked
/// by a unit test (see __tests__/PublisherKeysPage.test.tsx) against
/// known NIST vectors.
function sha256Fallback(bytes: Uint8Array): Uint8Array {
  const K = new Uint32Array([
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
    0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
  ]);
  const H = new Uint32Array([
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
    0x1f83d9ab, 0x5be0cd19,
  ]);
  // Padding: append 0x80 then zeros then 64-bit length (big-endian).
  const l = bytes.length;
  const bitLen = l * 8;
  const padLen = (l + 9 + 63) & ~63; // next multiple of 64 with room for 1+8
  const padded = new Uint8Array(padLen);
  padded.set(bytes);
  padded[l] = 0x80;
  // Write bit length in the last 8 bytes as big-endian 64-bit.
  const hi = Math.floor(bitLen / 0x100000000);
  const lo = bitLen >>> 0;
  const dv = new DataView(padded.buffer);
  dv.setUint32(padLen - 8, hi);
  dv.setUint32(padLen - 4, lo);

  const w = new Uint32Array(64);
  for (let chunk = 0; chunk < padLen; chunk += 64) {
    for (let i = 0; i < 16; i++) w[i] = dv.getUint32(chunk + i * 4);
    for (let i = 16; i < 64; i++) {
      const s0 = ((w[i - 15] >>> 7) | (w[i - 15] << 25)) ^
                 ((w[i - 15] >>> 18) | (w[i - 15] << 14)) ^
                 (w[i - 15] >>> 3);
      const s1 = ((w[i - 2] >>> 17) | (w[i - 2] << 15)) ^
                 ((w[i - 2] >>> 19) | (w[i - 2] << 13)) ^
                 (w[i - 2] >>> 10);
      w[i] = (w[i - 16] + s0 + w[i - 7] + s1) >>> 0;
    }
    let [a, b, c, d, e, f, g, h] = [
      H[0], H[1], H[2], H[3], H[4], H[5], H[6], H[7],
    ];
    for (let i = 0; i < 64; i++) {
      const S1 = ((e >>> 6) | (e << 26)) ^ ((e >>> 11) | (e << 21)) ^ ((e >>> 25) | (e << 7));
      const ch = (e & f) ^ (~e & g);
      const t1 = (h + S1 + ch + K[i] + w[i]) >>> 0;
      const S0 = ((a >>> 2) | (a << 30)) ^ ((a >>> 13) | (a << 19)) ^ ((a >>> 22) | (a << 10));
      const mj = (a & b) ^ (a & c) ^ (b & c);
      const t2 = (S0 + mj) >>> 0;
      h = g;
      g = f;
      f = e;
      e = (d + t1) >>> 0;
      d = c;
      c = b;
      b = a;
      a = (t1 + t2) >>> 0;
    }
    H[0] = (H[0] + a) >>> 0;
    H[1] = (H[1] + b) >>> 0;
    H[2] = (H[2] + c) >>> 0;
    H[3] = (H[3] + d) >>> 0;
    H[4] = (H[4] + e) >>> 0;
    H[5] = (H[5] + f) >>> 0;
    H[6] = (H[6] + g) >>> 0;
    H[7] = (H[7] + h) >>> 0;
  }
  const out = new Uint8Array(32);
  const outDv = new DataView(out.buffer);
  for (let i = 0; i < 8; i++) outDv.setUint32(i * 4, H[i]);
  return out;
}

// Exported for unit tests.
export const __testing__ = { sha256, sha256Fallback, normalizeBase64 };

const PublisherKeysPage: React.FC = () => {
  const queryClient = useQueryClient();
  const [showAdd, setShowAdd] = useState(false);
  const [label, setLabel] = useState('');
  const [publicKeyB64, setPublicKeyB64] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [confirmRevoke, setConfirmRevoke] = useState<PublicKey | null>(null);
  const [fingerprints, setFingerprints] = useState<Record<string, string>>({});

  const {
    data: keys,
    isLoading,
    isError,
    error: queryError,
  } = useQuery({
    queryKey: ['publisher-public-keys'],
    queryFn: fetchKeys,
  });

  // Derive fingerprints whenever the key list changes. Done in an
  // effect so we don't block render while waiting for SubtleCrypto.
  React.useEffect(() => {
    if (!keys) return;
    let cancelled = false;
    Promise.all(keys.map((k) => fingerprint(k.publicKeyB64))).then((fps) => {
      if (cancelled) return;
      const next: Record<string, string> = {};
      keys.forEach((k, i) => {
        next[k.id] = fps[i];
      });
      setFingerprints(next);
    });
    return () => {
      cancelled = true;
    };
  }, [keys]);

  const addMutation = useMutation({
    mutationFn: addKey,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['publisher-public-keys'] });
      setShowAdd(false);
      setLabel('');
      setPublicKeyB64('');
      setError(null);
    },
    onError: (e: Error) => setError(e.message),
  });

  const revokeMutation = useMutation({
    mutationFn: revokeKey,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['publisher-public-keys'] });
      setConfirmRevoke(null);
    },
    onError: (e: Error) => setError(e.message),
  });

  const handleAddSubmit = () => {
    setError(null);
    const trimmedLabel = label.trim();
    if (!trimmedLabel) {
      setError('Label is required.');
      return;
    }
    const trimmedKey = publicKeyB64.trim();
    if (!trimmedKey) {
      setError('Public key is required.');
      return;
    }
    // Quick client-side length check so obvious typos never leave the
    // browser. 32-byte Ed25519 pubkey = 44 b64 chars (with padding) or
    // 43 (URL-safe, no padding).
    const stripped = normalizeBase64(trimmedKey);
    if (stripped.length !== 44) {
      setError(
        `Expected 44 base64 characters for an Ed25519 public key; got ${stripped.length}.`
      );
      return;
    }
    addMutation.mutate({
      algorithm: 'ed25519',
      publicKeyB64: trimmedKey,
      label: trimmedLabel,
    });
  };

  return (
    <Paper sx={{ p: 3, m: 2 }}>
      <Stack direction="row" alignItems="center" justifyContent="space-between" mb={2}>
        <Stack direction="row" alignItems="center" spacing={1}>
          <KeyIcon color="primary" />
          <Typography variant="h5">Publisher Attestation Keys</Typography>
        </Stack>
        <Button
          variant="contained"
          startIcon={<AddIcon />}
          onClick={() => setShowAdd(true)}
          disabled={addMutation.isPending}
        >
          Add key
        </Button>
      </Stack>

      <Typography variant="body2" color="text.secondary" mb={3}>
        Register an Ed25519 public key to sign SBOMs at publish time. The
        registry verifies signatures against any of your active keys and
        surfaces a <b>verified publisher attestation</b> finding on each
        accepted version. Generate a keypair locally with{' '}
        <code>mockforge-plugin key gen</code> — the private half never
        leaves your machine.
      </Typography>

      {isLoading && <LinearProgress />}
      {isError && (
        <Typography color="error" variant="body2" mb={2}>
          {(queryError as Error)?.message ?? 'Failed to load keys.'}
        </Typography>
      )}

      {keys && keys.length === 0 && !isLoading && (
        <Card variant="outlined">
          <CardContent>
            <Typography variant="body2" color="text.secondary">
              No public keys registered yet. Click <b>Add key</b> above to
              register one.
            </Typography>
          </CardContent>
        </Card>
      )}

      <Stack spacing={2}>
        {keys?.map((key) => (
          <Card key={key.id} variant="outlined">
            <CardContent>
              <Stack direction="row" alignItems="center" spacing={2}>
                <VerifiedIcon
                  color={key.revokedAt ? 'disabled' : 'success'}
                  fontSize="large"
                />
                <div style={{ flexGrow: 1 }}>
                  <Stack direction="row" alignItems="center" spacing={1} mb={0.5}>
                    <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                      {key.label}
                    </Typography>
                    <Chip
                      label={key.algorithm}
                      size="small"
                      color="primary"
                      variant="outlined"
                    />
                    {key.revokedAt && (
                      <Chip label="Revoked" color="default" size="small" />
                    )}
                  </Stack>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ display: 'block' }}
                  >
                    id: <code>{key.id}</code>
                  </Typography>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ display: 'block' }}
                  >
                    fingerprint:{' '}
                    <code>{fingerprints[key.id] ?? 'computing…'}</code>
                  </Typography>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    sx={{ display: 'block' }}
                  >
                    created: {new Date(key.createdAt).toLocaleString()}
                    {key.revokedAt &&
                      ` • revoked ${new Date(key.revokedAt).toLocaleString()}`}
                  </Typography>
                </div>
                <Tooltip title="Copy public key">
                  <IconButton
                    onClick={() => navigator.clipboard.writeText(key.publicKeyB64)}
                  >
                    <CopyIcon />
                  </IconButton>
                </Tooltip>
                {!key.revokedAt && (
                  <Tooltip title="Revoke key">
                    <IconButton
                      onClick={() => setConfirmRevoke(key)}
                      disabled={revokeMutation.isPending}
                    >
                      <DeleteIcon />
                    </IconButton>
                  </Tooltip>
                )}
              </Stack>
            </CardContent>
          </Card>
        ))}
      </Stack>

      {/* Add-key dialog */}
      <Dialog open={showAdd} onClose={() => setShowAdd(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Register a public key</DialogTitle>
        <DialogContent>
          <DialogContentText sx={{ mb: 2 }}>
            Paste the base64-encoded Ed25519 public key produced by{' '}
            <code>mockforge-plugin key gen</code> or{' '}
            <code>openssl pkey -in key.pem -pubout</code>.
          </DialogContentText>
          <Stack spacing={2}>
            <TextField
              label="Label"
              placeholder="e.g. laptop, ci-2026"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              inputProps={{ maxLength: 128 }}
              fullWidth
              autoFocus
            />
            <TextField
              label="Public key (base64)"
              value={publicKeyB64}
              onChange={(e) => setPublicKeyB64(e.target.value)}
              multiline
              minRows={2}
              fullWidth
              placeholder="AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
            />
            {error && (
              <Typography color="error" variant="body2">
                {error}
              </Typography>
            )}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setShowAdd(false)} disabled={addMutation.isPending}>
            Cancel
          </Button>
          <Button
            variant="contained"
            onClick={handleAddSubmit}
            disabled={addMutation.isPending}
          >
            {addMutation.isPending ? 'Registering…' : 'Register'}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Revoke confirmation */}
      <Dialog open={!!confirmRevoke} onClose={() => setConfirmRevoke(null)}>
        <DialogTitle>Revoke key?</DialogTitle>
        <DialogContent>
          <DialogContentText>
            Revoking <b>{confirmRevoke?.label}</b> marks the key inactive
            immediately. New publish signatures made with this key will
            fail verification. Historical attestations remain intact.
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button
            onClick={() => setConfirmRevoke(null)}
            disabled={revokeMutation.isPending}
          >
            Cancel
          </Button>
          <Button
            color="error"
            variant="contained"
            onClick={() => confirmRevoke && revokeMutation.mutate(confirmRevoke.id)}
            disabled={revokeMutation.isPending}
          >
            {revokeMutation.isPending ? 'Revoking…' : 'Revoke'}
          </Button>
        </DialogActions>
      </Dialog>
    </Paper>
  );
};

export default PublisherKeysPage;
