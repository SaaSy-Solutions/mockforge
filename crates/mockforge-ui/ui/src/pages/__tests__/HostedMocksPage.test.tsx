import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { HostedMocksPage } from '../HostedMocksPage';

vi.mock('../../services/tokenStorage', () => ({ getAuthToken: () => 'synthetic-test-token' }));

async function openDeployDialog() {
  render(<MemoryRouter><HostedMocksPage /></MemoryRouter>);
  await userEvent.click(await screen.findByRole('button', { name: 'Deploy Mock' }));
  return screen.getByRole('dialog');
}

describe('Hosted mock deployment form', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
    vi.mocked(fetch).mockResolvedValue(new Response(JSON.stringify([]), { status: 200 }));
  });

  it('exposes deployment column headers to assistive technology', async () => {
    render(<MemoryRouter><HostedMocksPage /></MemoryRouter>);
    for (const name of ['Name', 'Status', 'Health', 'URL', 'Created', 'Actions']) {
      expect(await screen.findByRole('columnheader', { name })).toHaveAttribute('scope', 'col');
    }
  });

  it('opens the spec picker with a native keyboard-accessible button', async () => {
    const dialog = await openDeployDialog();
    const upload = within(dialog).getByRole('button', { name: 'Upload' });
    const input = dialog.querySelector<HTMLInputElement>('input[type="file"]')!;
    const picker = vi.spyOn(input, 'click').mockImplementation(() => {});
    expect(upload.tagName).toBe('BUTTON');
    expect(upload).toHaveAttribute('type', 'button');
    expect(upload.contains(input)).toBe(false);
    upload.focus();
    await userEvent.keyboard('{Enter}');
    await userEvent.keyboard(' ');
    expect(picker).toHaveBeenCalledTimes(2);
    expect(fetch).not.toHaveBeenCalledWith('/api/v1/hosted-mocks/specs/upload', expect.anything());
  });

  it('prevents opening another picker during upload and recovers after failure', async () => {
    const dialog = await openDeployDialog();
    const input = dialog.querySelector<HTMLInputElement>('input[type="file"]')!;
    const file = new File(['{}'], 'spec.json', { type: 'application/json' });
    let finish!: (response: Response) => void;
    vi.mocked(fetch).mockImplementation(async (url) => {
      if (url === '/api/v1/hosted-mocks/specs/upload') {
        return new Promise<Response>((resolve) => { finish = resolve; });
      }
      return new Response(JSON.stringify([]), { status: 200 });
    });
    fireEvent.change(input, { target: { files: [file] } });
    const upload = await within(dialog).findByRole('button', { name: 'Uploading…' });
    expect(upload).toBeDisabled();
    const picker = vi.spyOn(input, 'click').mockImplementation(() => {});
    await userEvent.click(upload);
    expect(picker).not.toHaveBeenCalled();
    expect(input.value).toBe('');
    finish(new Response(JSON.stringify({ error: 'Spec could not be stored' }), { status: 500 }));
    await waitFor(() => expect(within(dialog).getByRole('button', { name: 'Upload' })).toBeEnabled());
    expect(await within(dialog).findByText('Spec could not be stored')).toBeVisible();
  });

  it('exposes the project and region selects with their labels and preserves a changed region', async () => {
    const dialog = await openDeployDialog();
    expect(within(dialog).getByRole('combobox', { name: 'Project (optional)' })).toBeVisible();
    const region = within(dialog).getByRole('combobox', { name: 'Region' });
    expect(region).toHaveValue('iad');
    await userEvent.selectOptions(region, 'lhr');
    expect(region).toHaveValue('lhr');
    expect(within(region).getAllByRole('option')).toHaveLength(12);
  });
});
