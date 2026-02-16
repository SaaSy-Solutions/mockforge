/**
 * @jest-environment jsdom
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Input } from '../../ui/input';

describe('UI Components', () => {
  it('renders input placeholder and value', () => {
    render(<Input placeholder="Search" defaultValue="abc" />);
    expect(screen.getByPlaceholderText('Search')).toBeInTheDocument();
    expect(screen.getByDisplayValue('abc')).toBeInTheDocument();
  });
});
