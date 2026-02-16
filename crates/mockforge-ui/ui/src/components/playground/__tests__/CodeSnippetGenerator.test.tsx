import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { CodeSnippetGenerator } from '../CodeSnippetGenerator';
import { usePlaygroundStore } from '../../../stores/usePlaygroundStore';
import { apiService } from '../../../services/api';

// Mock the stores and services
vi.mock('../../../stores/usePlaygroundStore');
vi.mock('../../../services/api');

describe('CodeSnippetGenerator', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders empty state when no request is configured', () => {
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
    });

    (apiService.generateCodeSnippet as any) = vi.fn().mockResolvedValue({
      snippets: {},
    });

    render(<CodeSnippetGenerator />);

    expect(screen.getByText(/Generating snippets.../i)).toBeInTheDocument();
  });

  it('generates snippets for REST request', async () => {
    (usePlaygroundStore as any).mockReturnValue({
      protocol: 'rest',
      restRequest: {
        method: 'POST',
        path: '/api/users',
        headers: { 'Content-Type': 'application/json' },
        body: '{"name":"John"}',
        base_url: 'http://localhost:3000',
      },
      graphQLRequest: {
        query: '',
        variables: {},
      },
    });

    (apiService.generateCodeSnippet as any) = vi.fn().mockResolvedValue({
      snippets: {
        curl: 'curl -X POST ...',
        javascript: 'fetch(...)',
        python: 'import requests\n...',
      },
    });

    render(<CodeSnippetGenerator />);

    await waitFor(() => {
      expect(apiService.generateCodeSnippet).toHaveBeenCalled();
    });
  });

  it('generates snippets for GraphQL request', async () => {
    (usePlaygroundStore as any).mockReturnValue({
      protocol: 'graphql',
      restRequest: {
        method: 'GET',
        path: '',
        headers: {},
        body: '',
      },
      graphQLRequest: {
        query: 'query { user(id: 1) { name } }',
        variables: {},
        base_url: 'http://localhost:4000',
      },
    });

    (apiService.generateCodeSnippet as any) = vi.fn().mockResolvedValue({
      snippets: {
        curl: 'curl -X POST ...',
        javascript: 'fetch(...)',
      },
    });

    render(<CodeSnippetGenerator />);

    await waitFor(() => {
      expect(apiService.generateCodeSnippet).toHaveBeenCalled();
    });
  });
});
