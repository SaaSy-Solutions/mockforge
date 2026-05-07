/**
 * @jest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';

// `TestingPage` now branches on `isCloudMode()` (#392). The shared test
// setup pins `VITE_API_BASE_URL`, which legacy-detects as cloud mode —
// this mock forces local mode so the existing assertions keep covering
// the self-hosted Smoke / Health-Check / Integration tabs. A separate
// suite would be needed to cover the cloud branch (deployment picker +
// SSE event panel) since it requires fetch + EventSource mocks.
const cloudModeMock = vi.hoisted(() => ({
  isCloudMode: vi.fn(() => false),
  getCloudApiBase: vi.fn(() => ''),
}));
vi.mock('../../../utils/cloudMode', () => cloudModeMock);

import { TestingPage } from '../../../pages/TestingPage';

describe('Testing Components', () => {
  beforeEach(() => {
    cloudModeMock.isCloudMode.mockReturnValue(false);
  });

  it('renders testing suites on initial load', () => {
    render(<TestingPage />);
    expect(screen.getByText('Testing Suite')).toBeInTheDocument();
    expect(screen.getByText('Smoke Tests')).toBeInTheDocument();
    expect(screen.getByText('Health Check')).toBeInTheDocument();
  });
});
