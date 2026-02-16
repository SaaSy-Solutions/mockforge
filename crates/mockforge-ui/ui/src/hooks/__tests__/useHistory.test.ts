/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useHistory } from '../useHistory';

describe('useHistory', () => {
  it('should initialize with initial state', () => {
    const initialState = { nodes: [], edges: [] };
    const { result } = renderHook(() => useHistory(initialState));

    expect(result.current.history).toEqual(initialState);
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(false);
  });

  it('should push new state to history', () => {
    const initialState = { nodes: [], edges: [] };
    const { result } = renderHook(() => useHistory(initialState));

    const newState = { nodes: [{ id: '1' }], edges: [] };

    act(() => {
      result.current.push(newState);
    });

    expect(result.current.history).toEqual(newState);
    expect(result.current.canUndo).toBe(true);
    expect(result.current.canRedo).toBe(false);
  });

  it('should undo to previous state', () => {
    const initialState = { nodes: [], edges: [] };
    const { result } = renderHook(() => useHistory(initialState));

    const newState = { nodes: [{ id: '1' }], edges: [] };

    act(() => {
      result.current.push(newState);
    });

    act(() => {
      result.current.undo();
    });

    expect(result.current.history).toEqual(initialState);
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(true);
  });

  it('should redo to next state', () => {
    const initialState = { nodes: [], edges: [] };
    const { result } = renderHook(() => useHistory(initialState));

    const newState = { nodes: [{ id: '1' }], edges: [] };

    act(() => {
      result.current.push(newState);
    });

    act(() => {
      result.current.undo();
    });

    act(() => {
      result.current.redo();
    });

    expect(result.current.history).toEqual(newState);
    expect(result.current.canUndo).toBe(true);
    expect(result.current.canRedo).toBe(false);
  });

  it('should clear future when new state is pushed after undo', () => {
    const initialState = { nodes: [], edges: [] };
    const { result } = renderHook(() => useHistory(initialState));

    const state1 = { nodes: [{ id: '1' }], edges: [] };
    const state2 = { nodes: [{ id: '2' }], edges: [] };

    act(() => {
      result.current.push(state1);
      result.current.push(state2);
      result.current.undo();
      result.current.push({ nodes: [{ id: '3' }], edges: [] });
    });

    expect(result.current.canRedo).toBe(false);
  });

  it('should respect capacity limit', () => {
    const initialState = { nodes: [], edges: [] };
    const { result } = renderHook(() => useHistory(initialState, 2));

    act(() => {
      result.current.push({ nodes: [{ id: '1' }], edges: [] });
      result.current.push({ nodes: [{ id: '2' }], edges: [] });
      result.current.push({ nodes: [{ id: '3' }], edges: [] });
    });

    // Should only keep 2 past states
    act(() => {
      result.current.undo();
      result.current.undo();
    });

    // Should not be able to undo further
    expect(result.current.canUndo).toBe(false);
  });

  it('should clear history', () => {
    const initialState = { nodes: [], edges: [] };
    const { result } = renderHook(() => useHistory(initialState));

    act(() => {
      result.current.push({ nodes: [{ id: '1' }], edges: [] });
      result.current.clear();
    });

    expect(result.current.history).toEqual(initialState);
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(false);
  });
});
