/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ThemeToggle, SimpleThemeToggle } from '../ThemeToggle';
import { useThemePaletteStore } from '../../../stores/useThemePaletteStore';

vi.mock('../../../stores/useThemePaletteStore');

describe('ThemeToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useThemePaletteStore).mockReturnValue({
      theme: 'system',
      setTheme: vi.fn(),
    } as any);
  });

  it('renders all three theme buttons', () => {
    render(<ThemeToggle />);

    expect(screen.getByLabelText('Switch to light mode')).toBeInTheDocument();
    expect(screen.getByLabelText('Switch to system theme')).toBeInTheDocument();
    expect(screen.getByLabelText('Switch to dark mode')).toBeInTheDocument();
  });

  it('highlights current theme', () => {
    vi.mocked(useThemePaletteStore).mockReturnValue({
      theme: 'light',
      setTheme: vi.fn(),
    } as any);

    render(<ThemeToggle />);

    const lightButton = screen.getByLabelText('Switch to light mode');
    expect(lightButton).toHaveClass('bg-brand-50');
  });

  it('switches to light theme when clicked', () => {
    const setThemeMock = vi.fn();
    vi.mocked(useThemePaletteStore).mockReturnValue({
      theme: 'dark',
      setTheme: setThemeMock,
    } as any);

    render(<ThemeToggle />);

    fireEvent.click(screen.getByLabelText('Switch to light mode'));
    expect(setThemeMock).toHaveBeenCalledWith('light');
  });

  it('switches to dark theme when clicked', () => {
    const setThemeMock = vi.fn();
    vi.mocked(useThemePaletteStore).mockReturnValue({
      theme: 'light',
      setTheme: setThemeMock,
    } as any);

    render(<ThemeToggle />);

    fireEvent.click(screen.getByLabelText('Switch to dark mode'));
    expect(setThemeMock).toHaveBeenCalledWith('dark');
  });

  it('switches to system theme when clicked', () => {
    const setThemeMock = vi.fn();
    vi.mocked(useThemePaletteStore).mockReturnValue({
      theme: 'light',
      setTheme: setThemeMock,
    } as any);

    render(<ThemeToggle />);

    fireEvent.click(screen.getByLabelText('Switch to system theme'));
    expect(setThemeMock).toHaveBeenCalledWith('system');
  });

  it('applies custom className', () => {
    const { container } = render(<ThemeToggle className="custom-class" />);
    expect(container.firstChild).toHaveClass('custom-class');
  });

  it('renders with small size', () => {
    render(<ThemeToggle size="sm" />);
    const icons = document.querySelectorAll('.h-4');
    expect(icons.length).toBeGreaterThan(0);
  });
});

describe('SimpleThemeToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders moon icon in light mode', () => {
    vi.mocked(useThemePaletteStore).mockReturnValue({
      theme: 'light',
      setTheme: vi.fn(),
    } as any);

    render(<SimpleThemeToggle />);
    expect(screen.getByLabelText('Switch to dark mode')).toBeInTheDocument();
  });

  it('renders sun icon in dark mode', () => {
    vi.mocked(useThemePaletteStore).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
    } as any);

    render(<SimpleThemeToggle />);
    expect(screen.getByLabelText('Switch to light mode')).toBeInTheDocument();
  });

  it('toggles from light to dark', () => {
    const setThemeMock = vi.fn();
    vi.mocked(useThemePaletteStore).mockReturnValue({
      theme: 'light',
      setTheme: setThemeMock,
    } as any);

    render(<SimpleThemeToggle />);

    const toggleButton = screen.getByLabelText('Switch to dark mode');
    fireEvent.click(toggleButton);

    expect(setThemeMock).toHaveBeenCalledWith('dark');
  });

  it('toggles from dark to light', () => {
    const setThemeMock = vi.fn();
    vi.mocked(useThemePaletteStore).mockReturnValue({
      theme: 'dark',
      setTheme: setThemeMock,
    } as any);

    render(<SimpleThemeToggle />);

    const toggleButton = screen.getByLabelText('Switch to light mode');
    fireEvent.click(toggleButton);

    expect(setThemeMock).toHaveBeenCalledWith('light');
  });
});
