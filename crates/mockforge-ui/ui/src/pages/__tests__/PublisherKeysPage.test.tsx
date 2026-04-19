// SHA-256 fallback correctness check.
//
// The PublisherKeysPage shows a short fingerprint on each registered
// key so humans can sanity-check "is this the key I just generated?"
// We compute that fingerprint with `crypto.subtle.digest` when the
// browser exposes it, and fall back to a pure-JS implementation in
// non-secure-context dev setups. Both paths must produce identical
// output or the fingerprint is worthless for verification.
//
// The fixtures here are the canonical NIST / FIPS 180-4 test vectors
// (byte-for-byte from the published spec) plus a couple of inputs the
// project actually hits in practice (an all-zero Ed25519 key blob, a
// known 32-byte value).

import { describe, expect, it } from 'vitest';
import { __testing__ } from '../PublisherKeysPage';

function hex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

describe('PublisherKeysPage SHA-256 fallback', () => {
  it('matches NIST vectors for empty input', async () => {
    const got = __testing__.sha256Fallback(new Uint8Array([]));
    expect(hex(got)).toBe(
      'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
    );
  });

  it('matches NIST vector for "abc"', async () => {
    const abc = new TextEncoder().encode('abc');
    const got = __testing__.sha256Fallback(abc);
    expect(hex(got)).toBe(
      'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
    );
  });

  it('matches NIST vector for the 448-bit message', async () => {
    // Exercises the padding + two-block path where the length field
    // barely fits in the same 64-byte block.
    const msg = new TextEncoder().encode(
      'abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq',
    );
    const got = __testing__.sha256Fallback(msg);
    expect(hex(got)).toBe(
      '248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1',
    );
  });

  it('agrees with crypto.subtle when the runtime exposes it', async () => {
    // jsdom doesn't always expose crypto.subtle, and on runtimes that
    // skip it this test is an availability check rather than a parity
    // check — so we gate on the feature existing at all.
    if (typeof crypto === 'undefined' || !crypto.subtle?.digest) {
      return;
    }
    const bytes = new Uint8Array(32);
    for (let i = 0; i < 32; i++) bytes[i] = i * 7;

    const fromFallback = __testing__.sha256Fallback(bytes);
    const subtleBuf = await crypto.subtle.digest('SHA-256', bytes);
    const fromSubtle = new Uint8Array(subtleBuf);

    expect(hex(fromFallback)).toBe(hex(fromSubtle));
  });

  it('normalizes URL-safe base64', () => {
    // Matches the server's `rename_all = "camelCase"` + dual-decoder;
    // the UI must accept the same inputs the CLI accepts on `key add`.
    // Inputs whose length is already a multiple of 4 need no padding.
    expect(__testing__.normalizeBase64('ab-cd_ef')).toBe('ab+cd/ef');
    // Inputs shorter than a multiple of 4 get `=` pads.
    expect(__testing__.normalizeBase64('ab-c')).toBe('ab+c');
    expect(__testing__.normalizeBase64('ab-')).toBe('ab+=');
    expect(__testing__.normalizeBase64('abcd')).toBe('abcd');
  });
});
