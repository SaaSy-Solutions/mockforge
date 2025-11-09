import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { RequestPanel } from '../RequestPanel';
import { usePlaygroundStore } from '../../../stores/usePlaygroundStore';

// Mock the store
vi.mock('../../../stores/usePlaygroundStore');
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('RequestPanel', () => {
  const mockExecuteRestRequest = vi.fn();
  const mockExecuteGraphQLRequest = vi.fn();
  const mockSetRestRequest = vi.fn();
  const mockSetGraphQLRequest = vi.fn();
  const mockSetProtocol = vi.fn();
  const mockLoadEndpoints = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    (usePlaygroundStore as any).mockReturnValue({
      protocol: 'rest',
      restRequest: {
        method: 'GET',
        path: '',
        headers: {},
        body: '',
      },
      graphQLRequest: {
        query: '',
        variables: {},
      },
      executeRestRequest: mockExecuteRestRequest,
      executeGraphQLRequest: mockExecuteGraphQLRequest,
      setRestRequest: mockSetRestRequest,
      setGraphQLRequest: mockSetGraphQLRequest,
      setProtocol: mockSetProtocol,
      loadEndpoints: mockLoadEndpoints,
      responseLoading: false,
      endpoints: [],
    });
  });

  it('renders REST form by default', () => {
    render(<RequestPanel />);

    expect(screen.getByLabelText(/Method/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Path/i)).toBeInTheDocument();
  });

  it('switches to GraphQL protocol', () => {
    render(<RequestPanel />);

    const protocolSelect = screen.getByRole('combobox');
    fireEvent.change(protocolSelect, { target: { value: 'graphql' } });

    expect(mockSetProtocol).toHaveBeenCalledWith('graphql');
  });

  it('updates REST request path', () => {
    render(<RequestPanel />);

    const pathInput = screen.getByLabelText(/Path/i);
    fireEvent.change(pathInput, { target: { value: '/api/users' } });

    expect(mockSetRestRequest).toHaveBeenCalled();
  });

  it('executes REST request when execute button is clicked', async () => {
    (usePlaygroundStore as any).mockReturnValue({
      protocol: 'rest',
      restRequest: {
        method: 'GET',
        path: '/api/users',
        headers: {},
        body: '',
      },
      graphQLRequest: {
        query: '',
        variables: {},
      },
      executeRestRequest: mockExecuteRestRequest,
      executeGraphQLRequest: mockExecuteGraphQLRequest,
      setRestRequest: mockSetRestRequest,
      setGraphQLRequest: mockSetGraphQLRequest,
      setProtocol: mockSetProtocol,
      loadEndpoints: mockLoadEndpoints,
      responseLoading: false,
      endpoints: [],
    });

    render(<RequestPanel />);

    const executeButton = screen.getByRole('button', { name: /Execute/i });
    fireEvent.click(executeButton);

    await waitFor(() => {
      expect(mockExecuteRestRequest).toHaveBeenCalled();
    });
  });
});
