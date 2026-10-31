import 'fake-indexeddb/auto';
import { serialize, deserialize } from 'node:v8';

global.structuredClone = value => deserialize(serialize(value));
if (typeof globalThis.fetch !== 'function') throw new Error('Live tests require native Fetch');
window.fetch = globalThis.fetch.bind(globalThis);
