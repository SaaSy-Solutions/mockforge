import { describe, expect, it } from 'vitest';
import { bytesToBase64 } from '../binaryEncoding';

describe('marketplace binary file encoding', () => {
  it('encodes empty and non-UTF8 file bytes', () => {
    expect(bytesToBase64(new Uint8Array())).toBe('');
    expect(bytesToBase64(new Uint8Array([0, 1, 127, 128, 255]))).toBe('AAF/gP8=');
  });

  it('preserves a view’s offset and length', () => {
    expect(bytesToBase64(new Uint8Array([10, 0, 1, 127, 128, 255, 20]).subarray(1, 6)))
      .toBe('AAF/gP8=');
  });

  it('round-trips a 1 MiB file without exceeding the argument stack', () => {
    const bytes = Uint8Array.from({ length: 1024 * 1024 }, (_, index) => index % 256);
    const decoded = Uint8Array.from(atob(bytesToBase64(bytes)), (character) => character.charCodeAt(0));
    expect(decoded).toEqual(bytes);
  });
});
