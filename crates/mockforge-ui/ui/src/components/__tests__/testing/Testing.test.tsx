/**
 * @jest-environment jsdom
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TestingPage } from '../../../pages/TestingPage';

describe('Testing Components', () => {
  it('renders testing suites on initial load', () => {
    render(<TestingPage />);
    expect(screen.getByText('Testing Suite')).toBeInTheDocument();
    expect(screen.getByText('Smoke Tests')).toBeInTheDocument();
    expect(screen.getByText('Health Check')).toBeInTheDocument();
  });
});
