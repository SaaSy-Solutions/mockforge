import { describe, expect, it, vi } from 'vitest';
import { compareFidelity, parseRecorderSample } from '../fidelity';
import { fetchJsonWithErrorText } from '../client';
vi.mock('../client', () => ({ fetchJsonWithErrorText: vi.fn() }));

describe('Fidelity input and transport', () => {
  it('accepts actual recorder JSONL, arrays and single exchanges', () => {
    const exchange = { request: { method: 'GET' }, response: null };
    expect(parseRecorderSample(`${JSON.stringify(exchange)}\n${JSON.stringify(exchange)}\n`)).toEqual([exchange, exchange]);
    expect(parseRecorderSample(JSON.stringify([exchange]))).toEqual([exchange]);
    expect(parseRecorderSample(JSON.stringify(exchange))).toEqual([exchange]);
    expect(parseRecorderSample('')).toBeUndefined();
    expect(parseRecorderSample('[]')).toEqual([]);
    expect(() => parseRecorderSample('invalid export')).toThrow(/Flight Recorder/);
  });
  it('rejects stub or missing reports instead of presenting verification', async () => {
    vi.mocked(fetchJsonWithErrorText).mockResolvedValue({});
    await expect(compareFidelity('mock', 'source', undefined, new AbortController().signal)).rejects.toThrow(/report was not returned/);
  });
});
