import { useEffect, useRef, useState, type FormEvent } from 'react';
import { compareFidelity, parseRecorderSample, type FidelityAssessment } from '../../services/api/fidelity';

const MAX_BYTES = 2 * 1024 * 1024;
type Field = 'mock' | 'source' | 'sample';

export function useFidelityComparison() {
  const [inputs, setInputs] = useState({ mock: '', source: '', sample: '' });
  const [report, setReport] = useState<FidelityAssessment | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const generation = useRef(0);
  const controller = useRef<AbortController | null>(null);
  useEffect(() => () => { generation.current += 1; controller.current?.abort(); }, []);

  function clear() {
    generation.current += 1;
    controller.current?.abort();
    setReport(null); setError(null); setPending(false);
    return generation.current;
  }

  function update(field: Field, text: string) {
    clear();
    setInputs(previous => ({ ...previous, [field]: text }));
  }

  async function load(file: File | undefined, field: Field) {
    if (!file) return;
    const current = clear();
    if (file.size > MAX_BYTES) { setError('Choose a file smaller than 2 MiB.'); return; }
    try {
      const text = await file.text();
      if (current === generation.current) update(field, text);
    } catch {
      if (current === generation.current) setError('The file could not be read. Choose it again or paste its contents.');
    }
  }

  async function compare(event: FormEvent) {
    event.preventDefault();
    const current = clear();
    controller.current = new AbortController();
    setPending(true);
    try {
      if (Object.values(inputs).some(text => new Blob([text]).size > MAX_BYTES)) {
        throw new Error('Each input must be smaller than 2 MiB.');
      }
      const result = await compareFidelity(inputs.mock, inputs.source, parseRecorderSample(inputs.sample), controller.current.signal);
      if (current === generation.current) setReport(result);
    } catch (err) {
      if (current === generation.current) setError(err instanceof Error ? err.message : 'Comparison failed. Try again.');
    } finally { if (current === generation.current) setPending(false); }
  }

  return { ...inputs, report, error, pending, update, load, compare };
}
