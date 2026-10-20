import { useEffect, useRef, useState } from 'react';
import { Button } from '../ui/button';
import { Textarea } from '../ui/textarea';
import { Label } from '../ui/label';
import { Input } from '../ui/input';
import { ModernBadge } from '../ui/design-system/Badge';
import { Section } from '../ui/design-system/PageHeader';
import { compareFidelity, parseRecorderSample, type FidelityAssessment } from '../../services/api/fidelity';
import { FidelityResults } from './FidelityResults';

const MAX_BYTES = 2 * 1024 * 1024;

export function FidelityPanel() {
  const [mock, setMock] = useState('');
  const [source, setSource] = useState('');
  const [sample, setSample] = useState('');
  const [report, setReport] = useState<FidelityAssessment | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const generation = useRef(0);
  const controller = useRef<AbortController | null>(null);
  useEffect(() => () => { generation.current += 1; controller.current?.abort(); }, []);

  function update(set: (text: string) => void, text: string) {
    generation.current += 1;
    controller.current?.abort();
    set(text); setReport(null); setError(null); setPending(false);
  }

  async function load(file: File | undefined, set: (text: string) => void) {
    if (!file) return;
    const current = ++generation.current; controller.current?.abort();
    setReport(null); setError(null); setPending(false);
    if (file.size > MAX_BYTES) { setError('Choose a file smaller than 2 MiB.'); return; }
    try {
      const text = await file.text();
      if (current === generation.current) update(set, text);
    } catch {
      if (current === generation.current) setError('The file could not be read. Choose it again or paste its contents.');
    }
  }

  async function compare(event: React.FormEvent) {
    event.preventDefault();
    const current = ++generation.current;
    controller.current?.abort();
    controller.current = new AbortController();
    setPending(true); setError(null); setReport(null);
    try {
      if ([mock, source, sample].some(text => new Blob([text]).size > MAX_BYTES)) {
        throw new Error('Each input must be smaller than 2 MiB.');
      }
      const result = await compareFidelity(mock, source, parseRecorderSample(sample), controller.current.signal);
      if (current === generation.current) setReport(result);
    } catch (err) {
      if (current === generation.current) setError(err instanceof Error ? err.message : 'Comparison failed. Try again.');
    } finally { if (current === generation.current) setPending(false); }
  }

  const sampleDrift = report?.sample?.routes.filter(route => route.status === 'drift-detected').length ?? 0;
  const sampleIncomplete = report?.sample?.routes.filter(route => route.status !== 'verified' && route.status !== 'drift-detected').length ?? 0;

  return <Section title="Mock fidelity" subtitle="Compare the definition behind your mock with its current API contract.">
    <form onSubmit={compare} className="space-y-4" aria-busy={pending}>
      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          <Label htmlFor="fidelity-mock">Mock definition</Label>
          <Input type="file" accept=".json,.yaml,.yml" aria-label="Load mock definition" onChange={e => void load(e.target.files?.[0], setMock)} />
          <Textarea id="fidelity-mock" required rows={6} value={mock} onChange={e => update(setMock, e.target.value)} placeholder="Paste the OpenAPI definition used to generate the mock" />
        </div>
        <div className="space-y-2">
          <Label htmlFor="fidelity-source">Current source contract</Label>
          <Input type="file" accept=".json,.yaml,.yml" aria-label="Load current source contract" onChange={e => void load(e.target.files?.[0], setSource)} />
          <Textarea id="fidelity-source" required rows={6} value={source} onChange={e => update(setSource, e.target.value)} placeholder="Paste the current upstream OpenAPI contract" />
        </div>
      </div>
      <p className="text-sm text-muted-foreground">OpenAPI or Swagger 2, JSON or YAML. Up to 2 MiB per input.</p>
      <details>
        <summary className="cursor-pointer text-sm font-medium focus-visible:outline focus-visible:outline-2">Include a recorded upstream sample</summary>
        <div className="mt-3 space-y-2">
          <Label htmlFor="fidelity-sample">Flight Recorder export (optional)</Label>
          <Input type="file" accept=".json,.jsonl" aria-label="Load recorded sample" onChange={e => void load(e.target.files?.[0], setSample)} />
          <Textarea id="fidelity-sample" rows={4} value={sample} onChange={e => update(setSample, e.target.value)} placeholder="Paste a JSONL export or JSON array of recorded exchanges" />
          <p className="text-sm text-muted-foreground">Use a fresh HTTP capture, up to 500 exchanges. Routes without traffic remain not sampled. Unsupported sample media remains not verified.</p>
        </div>
      </details>
      <Button type="submit" disabled={pending || !mock.trim() || !source.trim()}>{pending ? 'Comparing…' : 'Compare fidelity'}</Button>
      {pending && <p role="status" className="text-sm text-muted-foreground">Checking route definitions and recorded traffic…</p>}
      {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
    </form>
    {report && <div className="mt-6 space-y-3">
      <p role="status" className="text-sm">{report.definition.routes.length} route definitions compared. <ModernBadge variant={report.definition.routes_with_drift ? 'warning' : 'outline'}>{report.definition.routes_with_drift} definitions with drift</ModernBadge></p>
      <p className="text-sm text-muted-foreground">{report.sample
        ? `${report.sample.exchanges_checked} recorded exchanges checked; latest capture ${new Date(report.sample.latest_capture_at).toLocaleString()}. Results cover only the supplied definitions and sample.`
        : 'Definition comparison only. Include a recorder export to check observed upstream traffic.'}</p>
      {report.sample && <p role="status" className="flex flex-wrap gap-2 text-sm">
        {sampleDrift > 0 && <ModernBadge variant="error">Sample drift: {sampleDrift} routes</ModernBadge>}
        {sampleIncomplete > 0 && <ModernBadge variant="warning">Sample coverage incomplete: {sampleIncomplete} routes</ModernBadge>}
        {sampleDrift === 0 && sampleIncomplete === 0 && <ModernBadge variant="success">Sampled routes verified</ModernBadge>}
      </p>}
      <FidelityResults report={report} />
    </div>}
  </Section>;
}
