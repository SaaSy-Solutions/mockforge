import { useState } from 'react';
import { Button } from '../ui/button';
import { ModernBadge } from '../ui/design-system/Badge';
import type { FidelityAssessment } from '../../services/api/fidelity';

function Status({ status }: { status: string }) {
  const label = status === 'verified' ? 'Verified' : status === 'drift-detected' ? 'Drift detected'
    : status === 'not-sampled' ? 'Not sampled' : status === 'not-verified' ? 'Not verified' : 'Not declared';
  return <ModernBadge variant={status === 'verified' ? 'success' : status === 'drift-detected' ? 'warning' : 'outline'}>{label}</ModernBadge>;
}

export function FidelityResults({ report }: { report: FidelityAssessment }) {
  const [page, setPage] = useState(0);
  const [open, setOpen] = useState<string | null>(null);
  const definitions = new Map(report.definition.routes.map(route => [route.endpoint, route]));
  const samples = new Map(report.sample?.routes.map(route => [route.endpoint, route]) ?? []);
  const endpoints = [...new Set([...definitions.keys(), ...samples.keys()])].sort();
  const pageCount = Math.max(1, Math.ceil(endpoints.length / 50));
  const shown = endpoints.slice(page * 50, (page + 1) * 50);
  return <>
    <div className="overflow-x-auto">
      <table className="w-full text-left text-sm">
        <caption className="sr-only">Per-route fidelity for the supplied definitions and recorded sample</caption>
        <thead><tr className="border-b border-border"><th className="p-2">Route</th><th className="p-2">Definition</th>{report.sample && <th className="p-2">Recorded sample</th>}<th className="p-2">Details</th></tr></thead>
        <tbody>{shown.map(endpoint => {
          const definition = definitions.get(endpoint);
          const sample = samples.get(endpoint);
          const details = (definition?.deltas.length ?? 0) + (sample?.findings.length ?? 0) + (sample?.notes.length ?? 0);
          return <tr key={endpoint} className="border-b border-border align-top">
            <th scope="row" className="break-all p-2 font-mono text-xs font-normal">{endpoint}</th>
            <td className="p-2"><Status status={definition?.status ?? 'absent'} />{definition && definition.change !== 'unchanged' && <p className="mt-1 text-xs text-muted-foreground">{definition.change}</p>}</td>
            {report.sample && <td className="p-2"><Status status={sample?.status ?? 'not-sampled'} /><p className="mt-1 text-xs text-muted-foreground">{sample?.exchanges_checked ?? 0} exchanges</p></td>}
            <td className="min-w-40 p-2">{details > 0 ? <>
              <Button type="button" size="sm" variant="ghost" aria-expanded={open === endpoint} onClick={() => setOpen(open === endpoint ? null : endpoint)}>{open === endpoint ? 'Hide details' : 'View details'}</Button>
              {open === endpoint && <div className="mt-2 max-w-xl space-y-3">
                {definition?.deltas.map((delta, index) => <div key={`${delta.location}-${index}`}>
                  <p className="break-all font-mono text-xs">{delta.location || 'Entire operation'} ({delta.change})</p>
                  <pre className="mt-1 max-h-64 overflow-auto whitespace-pre-wrap break-all rounded-lg bg-muted p-2 text-xs">{JSON.stringify({ mock: delta.mock, source: delta.source }, null, 2)}</pre>
                </div>)}
                {sample?.findings.map((finding, i) => <p key={`finding-${i}`} className="break-words text-xs">{finding}</p>)}
                {sample?.notes.map((note, i) => <p key={`note-${i}`} className="break-words text-xs">{note}</p>)}
              </div>}
            </> : <span className="text-muted-foreground">No differences</span>}</td>
          </tr>;
        })}</tbody>
      </table>
    </div>
    {pageCount > 1 && <div className="mt-3 flex items-center gap-3">
      <Button type="button" variant="outline" size="sm" disabled={page === 0} onClick={() => { setPage(page - 1); setOpen(null); }}>Previous</Button>
      <span className="text-sm">Page {page + 1} of {pageCount}</span>
      <Button type="button" variant="outline" size="sm" disabled={page + 1 >= pageCount} onClick={() => { setPage(page + 1); setOpen(null); }}>Next</Button>
    </div>}
  </>;
}
