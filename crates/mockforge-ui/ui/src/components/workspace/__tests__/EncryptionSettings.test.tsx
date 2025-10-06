/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import EncryptionSettings from '../EncryptionSettings';
import { apiService } from '../../../services/api';

vi.mock('../../../services/api');
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('EncryptionSettings', () => {
  const mockStatus = {
    enabled: false,
    masterKeySet: false,
    workspaceKeySet: false,
  };

  const mockConfig = {
    enabled: false,
    sensitiveHeaders: ['authorization', 'x-api-key'],
    sensitiveFields: ['password', 'token'],
    sensitiveEnvVars: ['API_KEY', 'SECRET_KEY'],
    customPatterns: [],
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (apiService.getWorkspaceEncryptionStatus as any) = vi.fn().mockResolvedValue(mockStatus);
    (apiService.getWorkspaceEncryptionConfig as any) = vi.fn().mockResolvedValue(mockConfig);
  });

  it('renders encryption settings header', () => {
    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    expect(screen.getByText('Encryption Settings')).toBeInTheDocument();
    expect(screen.getByText('Secure your workspace with end-to-end encryption')).toBeInTheDocument();
  });

  it('loads encryption status on mount', async () => {
    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => {
      expect(apiService.getWorkspaceEncryptionStatus).toHaveBeenCalledWith('ws-1');
      expect(apiService.getWorkspaceEncryptionConfig).toHaveBeenCalledWith('ws-1');
    });
  });

  it('displays not encrypted badge when disabled', async () => {
    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => {
      expect(screen.getByText('Not Encrypted')).toBeInTheDocument();
    });
  });

  it('displays encrypted badge when enabled', async () => {
    (apiService.getWorkspaceEncryptionStatus as any) = vi.fn().mockResolvedValue({
      ...mockStatus,
      enabled: true,
    });

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => {
      expect(screen.getByText('Encrypted')).toBeInTheDocument();
    });
  });

  it('shows master key status', async () => {
    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => {
      expect(screen.getByText('Master Key')).toBeInTheDocument();
      expect(screen.getByText('Not Set')).toBeInTheDocument();
    });
  });

  it('shows configured master key status', async () => {
    (apiService.getWorkspaceEncryptionStatus as any) = vi.fn().mockResolvedValue({
      ...mockStatus,
      masterKeySet: true,
    });

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => {
      expect(screen.getByText('Configured')).toBeInTheDocument();
    });
  });

  it('enables encryption when button clicked', async () => {
    (apiService.enableWorkspaceEncryption as any) = vi.fn().mockResolvedValue({});

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Enable Encryption'));

    fireEvent.click(screen.getByText('Enable Encryption'));

    await waitFor(() => {
      expect(apiService.enableWorkspaceEncryption).toHaveBeenCalledWith('ws-1');
    });
  });

  it('disables encryption with confirmation', async () => {
    global.confirm = vi.fn(() => true);
    (apiService.getWorkspaceEncryptionStatus as any) = vi.fn().mockResolvedValue({
      ...mockStatus,
      enabled: true,
    });
    (apiService.disableWorkspaceEncryption as any) = vi.fn().mockResolvedValue({});

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Disable Encryption'));

    fireEvent.click(screen.getByText('Disable Encryption'));

    await waitFor(() => {
      expect(global.confirm).toHaveBeenCalled();
      expect(apiService.disableWorkspaceEncryption).toHaveBeenCalledWith('ws-1');
    });
  });

  it('cancels disable when confirmation rejected', async () => {
    global.confirm = vi.fn(() => false);
    (apiService.getWorkspaceEncryptionStatus as any) = vi.fn().mockResolvedValue({
      ...mockStatus,
      enabled: true,
    });
    (apiService.disableWorkspaceEncryption as any) = vi.fn().mockResolvedValue({});

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Disable Encryption'));

    fireEvent.click(screen.getByText('Disable Encryption'));

    expect(apiService.disableWorkspaceEncryption).not.toHaveBeenCalled();
  });

  it('runs security check', async () => {
    const securityResult = {
      isSecure: true,
      warnings: [],
      errors: [],
      recommendations: [],
    };
    (apiService.checkWorkspaceSecurity as any) = vi.fn().mockResolvedValue(securityResult);

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Security Check'));

    fireEvent.click(screen.getByText('Security Check'));

    await waitFor(() => {
      expect(apiService.checkWorkspaceSecurity).toHaveBeenCalledWith('ws-1');
    });
  });

  it('switches to security tab', async () => {
    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Security'));

    fireEvent.click(screen.getByText('Security'));

    expect(screen.getByText('Security Analysis')).toBeInTheDocument();
  });

  it('displays security check results', async () => {
    const securityResult = {
      isSecure: false,
      warnings: [
        {
          severity: 'high',
          message: 'Unencrypted API key found',
          location: '/api/config',
          suggestion: 'Enable encryption',
        },
      ],
      errors: [],
      recommendations: ['Enable auto-encryption'],
    };
    (apiService.checkWorkspaceSecurity as any) = vi.fn().mockResolvedValue(securityResult);

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Security'));

    fireEvent.click(screen.getByText('Security'));
    fireEvent.click(screen.getByText('Run Security Check'));

    await waitFor(() => {
      expect(screen.getByText('Unencrypted API key found')).toBeInTheDocument();
      expect(screen.getByText('Enable auto-encryption')).toBeInTheDocument();
    });
  });

  it('shows backup key when available', async () => {
    (apiService.getWorkspaceEncryptionStatus as any) = vi.fn().mockResolvedValue({
      ...mockStatus,
      enabled: true,
      backupKey: 'ABC123-DEF456-GHI789',
    });

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => {
      expect(screen.getByText('Backup Key')).toBeInTheDocument();
    });
  });

  it('toggles backup key visibility', async () => {
    (apiService.getWorkspaceEncryptionStatus as any) = vi.fn().mockResolvedValue({
      ...mockStatus,
      enabled: true,
      backupKey: 'ABC123-DEF456-GHI789',
    });

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Backup Key'));

    const toggleButton = screen.getByRole('button', { name: '' }); // Eye icon button
    fireEvent.click(toggleButton);

    await waitFor(() => {
      expect(screen.getByText('ABC123-DEF456-GHI789')).toBeInTheDocument();
    });
  });

  it('copies backup key to clipboard', async () => {
    const clipboardWriteText = vi.fn();
    Object.assign(navigator, {
      clipboard: { writeText: clipboardWriteText },
    });

    (apiService.getWorkspaceEncryptionStatus as any) = vi.fn().mockResolvedValue({
      ...mockStatus,
      enabled: true,
      backupKey: 'ABC123-DEF456-GHI789',
    });

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Backup Key'));

    const copyButtons = screen.getAllByRole('button');
    const copyButton = copyButtons.find((btn) => btn.querySelector('svg')); // Copy icon
    fireEvent.click(copyButton!);

    expect(clipboardWriteText).toHaveBeenCalledWith('ABC123-DEF456-GHI789');
  });

  it('exports encrypted workspace', async () => {
    (apiService.getWorkspaceEncryptionStatus as any) = vi.fn().mockResolvedValue({
      ...mockStatus,
      enabled: true,
    });
    (apiService.exportWorkspaceEncrypted as any) = vi.fn().mockResolvedValue({});

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Export/Import'));

    fireEvent.click(screen.getByText('Export/Import'));
    fireEvent.click(screen.getByText('Export Encrypted Workspace'));

    const pathInput = screen.getByPlaceholderText('/path/to/workspace.enc');
    fireEvent.change(pathInput, { target: { value: '/export/path' } });

    const exportButtons = screen.getAllByText('Export');
    fireEvent.click(exportButtons[exportButtons.length - 1]);

    await waitFor(() => {
      expect(apiService.exportWorkspaceEncrypted).toHaveBeenCalledWith('ws-1', '/export/path');
    });
  });

  it('imports encrypted workspace', async () => {
    (apiService.importWorkspaceEncrypted as any) = vi.fn().mockResolvedValue({});

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Export/Import'));

    fireEvent.click(screen.getByText('Export/Import'));
    fireEvent.click(screen.getByText('Import Encrypted Workspace'));

    const pathInput = screen.getByPlaceholderText('/path/to/workspace.enc');
    fireEvent.change(pathInput, { target: { value: '/import/path' } });

    const keyInput = screen.getByPlaceholderText(/YKV2DK/);
    fireEvent.change(keyInput, { target: { value: 'BACKUP-KEY-123' } });

    fireEvent.click(screen.getByText('Import'));

    await waitFor(() => {
      expect(apiService.importWorkspaceEncrypted).toHaveBeenCalledWith(
        '/import/path',
        'ws-1',
        'BACKUP-KEY-123'
      );
    });
  });

  it('updates auto encryption configuration', async () => {
    (apiService.updateWorkspaceEncryptionConfig as any) = vi.fn().mockResolvedValue({});

    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Settings'));

    fireEvent.click(screen.getByText('Settings'));

    const textarea = screen.getAllByRole('textbox')[0];
    fireEvent.change(textarea, { target: { value: 'authorization\ncustom-header' } });

    fireEvent.click(screen.getByText('Save Settings'));

    await waitFor(() => {
      expect(apiService.updateWorkspaceEncryptionConfig).toHaveBeenCalledWith(
        'ws-1',
        expect.objectContaining({
          sensitiveHeaders: ['authorization', 'custom-header'],
        })
      );
    });
  });

  it('disables export when encryption not enabled', async () => {
    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => screen.getByText('Export/Import'));

    fireEvent.click(screen.getByText('Export/Import'));

    const exportButton = screen.getByText('Export Encrypted Workspace');
    expect(exportButton.closest('button')).toBeDisabled();
  });

  it('displays all encryption setting tabs', async () => {
    render(<EncryptionSettings workspaceId="ws-1" workspaceName="Test Workspace" />);

    await waitFor(() => {
      expect(screen.getByText('Overview')).toBeInTheDocument();
      expect(screen.getByText('Security')).toBeInTheDocument();
      expect(screen.getByText('Export/Import')).toBeInTheDocument();
      expect(screen.getByText('Settings')).toBeInTheDocument();
    });
  });
});
