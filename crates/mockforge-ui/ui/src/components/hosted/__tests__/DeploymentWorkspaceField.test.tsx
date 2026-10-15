import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DeploymentWorkspaceEditor, DeploymentWorkspaceSelect } from '../DeploymentWorkspaceField';
import { fetchJsonWithErrorBody } from '@/services/api/client';

vi.mock('@/services/api/client', () => ({ fetchJsonWithErrorBody: vi.fn() }));
const request = vi.mocked(fetchJsonWithErrorBody);
const workspaces = [{ id: 'workspace-a', name: 'Payments' }, { id: 'workspace-b', name: 'Orders' }];

beforeEach(() => { request.mockReset(); request.mockResolvedValue(workspaces); });

describe('Hosted deployment workspace selection', () => {
  it('keeps creation unassigned until the user chooses a workspace', async () => {
    const change = vi.fn();
    render(<DeploymentWorkspaceSelect value="" onChange={change} />);
    await screen.findByRole('option', { name: 'Payments' });
    expect(screen.getByRole('combobox', { name: /Workspace/ })).toHaveValue('');
    expect(change).not.toHaveBeenCalled();
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'workspace-a' } });
    expect(change).toHaveBeenCalledWith('workspace-a');
    expect(request).toHaveBeenCalledTimes(1);
  });

  it('saves only on explicit action and uses the deployment organization', async () => {
    const saved = vi.fn();
    render(<DeploymentWorkspaceEditor deploymentId="deployment-a" orgId="organization-a"
      workspaceId="workspace-a" onSaved={saved} />);
    await screen.findByRole('option', { name: 'Orders' });
    expect(request).toHaveBeenCalledTimes(1);
    expect(request.mock.calls[0][1]).toMatchObject({
      credentials: 'include', headers: { 'X-Organization-Id': 'organization-a' },
    });
    expect(screen.getByRole('button', { name: 'Save workspace' })).toBeDisabled();
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'workspace-b' } });
    expect(request).toHaveBeenCalledTimes(1);
    request.mockResolvedValueOnce({ id: 'deployment-a', workspace_id: 'workspace-b' });
    fireEvent.click(screen.getByRole('button', { name: 'Save workspace' }));
    await screen.findByText('Workspace saved');
    expect(saved).toHaveBeenCalledWith('workspace-b');
    expect(request.mock.calls[1]).toEqual(['/api/v1/hosted-mocks/deployment-a/workspace', {
      method: 'PATCH', credentials: 'include',
      headers: { 'Content-Type': 'application/json', 'X-Organization-Id': 'organization-a' },
      body: JSON.stringify({ workspace_id: 'workspace-b' }),
    }]);
    expect(request).toHaveBeenCalledTimes(2);
  });

  it('sends explicit null when detaching and prevents duplicate saves while pending', async () => {
    const saved = vi.fn();
    render(<DeploymentWorkspaceEditor deploymentId="deployment-a" orgId="organization-a"
      workspaceId="workspace-a" onSaved={saved} />);
    await screen.findByRole('option', { name: 'Payments' });
    fireEvent.change(screen.getByRole('combobox'), { target: { value: '' } });
    let resolve!: (value: unknown) => void;
    request.mockReturnValueOnce(new Promise(done => { resolve = done; }));
    fireEvent.click(screen.getByRole('button', { name: 'Save workspace' }));
    expect(screen.getByRole('button', { name: 'Saving workspace…' })).toBeDisabled();
    expect(screen.getByRole('combobox')).toBeDisabled();
    expect(JSON.parse(request.mock.calls[1][1]!.body as string)).toEqual({ workspace_id: null });
    resolve({ id: 'deployment-a', workspace_id: null });
    await screen.findByText('Workspace saved');
    expect(saved).toHaveBeenCalledWith(null);
  });

  it('keeps the selected draft and shows a denied save without claiming success', async () => {
    const saved = vi.fn();
    render(<DeploymentWorkspaceEditor deploymentId="deployment-a" orgId="organization-a"
      workspaceId="workspace-a" onSaved={saved} />);
    await screen.findByRole('option', { name: 'Orders' });
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'workspace-b' } });
    request.mockRejectedValueOnce(new Error('Access denied'));
    fireEvent.click(screen.getByRole('button', { name: 'Save workspace' }));
    await screen.findByText('Access denied');
    expect(screen.getByRole('combobox')).toHaveValue('workspace-b');
    expect(saved).not.toHaveBeenCalled();
    expect(screen.queryByText('Workspace saved')).not.toBeInTheDocument();
  });

  it('shows a list failure and retries without changing the assignment', async () => {
    request.mockRejectedValueOnce(new Error('Workspace service unavailable'));
    const change = vi.fn();
    render(<DeploymentWorkspaceSelect value="workspace-a" onChange={change} />);
    await screen.findByText('Workspace service unavailable');
    expect(screen.getByRole('combobox')).toHaveValue('workspace-a');
    fireEvent.click(screen.getByRole('button', { name: 'Retry workspaces' }));
    await screen.findByRole('option', { name: 'Payments' });
    expect(change).not.toHaveBeenCalled();
    expect(screen.queryByText('Workspace service unavailable')).not.toBeInTheDocument();
  });

  it('aborts the list request on unmount', async () => {
    request.mockReturnValueOnce(new Promise(() => {}));
    const { unmount } = render(<DeploymentWorkspaceSelect value="" onChange={vi.fn()} />);
    await waitFor(() => expect(request).toHaveBeenCalledTimes(1));
    const signal = request.mock.calls[0][1]!.signal!;
    expect(signal.aborted).toBe(false);
    unmount();
    expect(signal.aborted).toBe(true);
  });
});
