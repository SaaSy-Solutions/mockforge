import { useState } from 'react';

/** Shared request status; each recovery form owns its own inputs and validation. */
export function useRecoverySubmission() {
  const [pending, setPending] = useState(false);
  const [complete, setComplete] = useState(false);
  const [error, setError] = useState('');
  async function run(action: () => Promise<unknown>) {
    if (pending) return false;
    setPending(true); setError('');
    try {
      await action(); setComplete(true); return true;
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Could not complete your request. Please try again.');
      return false;
    } finally { setPending(false); }
  }
  return { pending, complete, error, run };
}
