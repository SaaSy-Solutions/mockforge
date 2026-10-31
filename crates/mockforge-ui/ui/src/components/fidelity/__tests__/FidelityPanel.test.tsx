import { fireEvent, render, screen, waitFor, within, act } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { FidelityPanel } from '../FidelityPanel';
import { FidelityResults } from '../FidelityResults';
import { compareFidelity, type FidelityAssessment } from '../../../services/api/fidelity';

vi.mock('../../../services/api/fidelity', async importOriginal => ({
  ...await importOriginal<typeof import('../../../services/api/fidelity')>(), compareFidelity: vi.fn(),
}));

const report: FidelityAssessment = {
  definition: { format_version: 1, routes_with_drift: 1, routes: [
    { endpoint: 'GET /items', status: 'verified', change: 'unchanged', deltas: [] },
    { endpoint: 'POST /other', status: 'drift-detected', change: 'changed', deltas: [
      { location: '/responses/200/schema/id/type', change: 'changed', mock: 'integer', source: 'string' },
    ] },
  ] },
  sample: { exchanges_checked: 1, latest_capture_at: '2026-10-20T00:00:00Z', routes: [
    { endpoint: 'GET /items', status: 'verified', exchanges_checked: 1, findings: [], notes: [] },
    { endpoint: 'POST /other', status: 'not-sampled', exchanges_checked: 0, findings: [], notes: [] },
  ] },
};
function fill() {
  fireEvent.change(screen.getByLabelText('Mock definition'), { target: { value: '{"mock":"definition"}' } });
  fireEvent.change(screen.getByLabelText('Current source contract'), { target: { value: '{"source":"definition"}' } });
}
beforeEach(() => { vi.mocked(compareFidelity).mockReset(); });

describe('Fidelity panel', () => {
  it('requires definitions and shows distinct route and sample badges with field details', async () => {
    vi.mocked(compareFidelity).mockResolvedValue(report);
    render(<FidelityPanel />);
    expect(screen.getByRole('button', { name: 'Compare fidelity' })).toBeDisabled();
    fill(); fireEvent.click(screen.getByRole('button', { name: 'Compare fidelity' }));
    const row = await screen.findByRole('row', { name: /POST \/other/ });
    expect(within(row).getByText('Drift detected')).toBeInTheDocument();
    expect(within(row).getByText('Not sampled')).toBeInTheDocument();
    expect(screen.queryByText(/\/responses\/200\/schema\/id\/type/)).not.toBeInTheDocument();
    fireEvent.click(within(row).getByRole('button', { name: 'View details' }));
    expect(screen.getByText(/\/responses\/200\/schema\/id\/type/)).toBeInTheDocument();
    expect(compareFidelity).toHaveBeenCalledTimes(1);
  });

  it.each(['drift-detected', 'not-sampled', 'not-verified'])('does not imply passing samples when definitions match but a route is %s', async status => {
    vi.mocked(compareFidelity).mockResolvedValue({
      definition: { format_version: 1, routes_with_drift: 0, routes: [report.definition.routes[0]] },
      sample: { exchanges_checked: 1, latest_capture_at: '2026-10-20T00:00:00Z', routes: [
        { endpoint: 'GET /items', status, exchanges_checked: status === 'not-sampled' ? 0 : 1, findings: [], notes: [] },
      ] },
    });
    render(<FidelityPanel />); fill();
    fireEvent.click(screen.getByRole('button', { name: 'Compare fidelity' }));
    expect(await screen.findByText(status === 'drift-detected' ? 'Sample drift: 1 routes' : 'Sample coverage incomplete: 1 routes')).toBeInTheDocument();
    expect(screen.queryByText('Sampled routes verified')).not.toBeInTheDocument();
    expect(screen.getByText('0 definitions with drift')).toBeInTheDocument();
  });

  it('preserves newer pasted input when an earlier file read finishes late', async () => {
    let resolve!: (value: string) => void;
    const file = new File(['old'], 'old.json', { type: 'application/json' });
    Object.defineProperty(file, 'text', { value: () => new Promise<string>(done => { resolve = done; }) });
    render(<FidelityPanel />); fill();
    fireEvent.change(screen.getByLabelText('Load mock definition'), { target: { files: [file] } });
    fireEvent.change(screen.getByLabelText('Mock definition'), { target: { value: 'newer pasted input' } });
    await act(async () => { resolve('old file contents'); });
    expect(screen.getByLabelText('Mock definition')).toHaveValue('newer pasted input');
  });

  it('clears old results when inputs change and ignores a late response', async () => {
    let resolve!: (value: FidelityAssessment) => void;
    vi.mocked(compareFidelity).mockReturnValue(new Promise(value => { resolve = value; }));
    render(<FidelityPanel />); fill();
    fireEvent.click(screen.getByRole('button', { name: 'Compare fidelity' }));
    expect(screen.getByText('Checking route definitions and recorded traffic…')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Current source contract'), { target: { value: 'new source' } });
    await act(async () => { resolve(report); });
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    expect(vi.mocked(compareFidelity).mock.calls[0][3].aborted).toBe(true);
    expect(screen.getByRole('button', { name: 'Compare fidelity' })).toBeEnabled();
  });

  it('shows API errors and permits retry without a passing report', async () => {
    vi.mocked(compareFidelity).mockRejectedValue(new Error('Invalid source contract. Check its JSON or YAML.'));
    render(<FidelityPanel />); fill();
    fireEvent.click(screen.getByRole('button', { name: 'Compare fidelity' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Invalid source contract');
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Compare fidelity' })).toBeEnabled();
  });

  it('removes a completed report after editing either definition', async () => {
    vi.mocked(compareFidelity).mockResolvedValue(report);
    render(<FidelityPanel />); fill();
    fireEvent.click(screen.getByRole('button', { name: 'Compare fidelity' }));
    await screen.findByRole('table');
    fireEvent.change(screen.getByLabelText('Mock definition'), { target: { value: 'changed' } });
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
  });

  it('paginates routes and renders expanded content only on demand', async () => {
    const many: FidelityAssessment = { definition: { format_version: 1, routes_with_drift: 0,
      routes: Array.from({ length: 52 }, (_, i) => ({ endpoint: `GET /route-${String(i).padStart(2, '0')}`, status: 'verified', change: 'unchanged', deltas: [] })) }, sample: null };
    render(<FidelityResults report={many} />);
    expect(screen.getAllByRole('row')).toHaveLength(51);
    expect(screen.queryByText('GET /route-51')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    await waitFor(() => expect(screen.getByText('GET /route-51')).toBeInTheDocument());
    expect(screen.getAllByRole('row')).toHaveLength(3);
  });
});
