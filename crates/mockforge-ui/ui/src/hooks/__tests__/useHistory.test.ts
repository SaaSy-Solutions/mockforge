/**
 * @vitest-environment jsdom
 */

import { describe, it, expect } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useHistory } from '../useHistory';

interface GraphState {
  nodes: { id: string }[];
  edges: { id: string }[];
}

describe('useHistory', () => {
  it('should initialize with initial state', () => {
    const initialState: GraphState = { nodes: [], edges: [] };
    const { result } = renderHook(() => useHistory(initialState));

    expect(result.current.history).toEqual(initialState);
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(false);
  });

  it('should push new state to history', () => {
    const initialState: GraphState = { nodes: [], edges: [] };
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
    const initialState: GraphState = { nodes: [], edges: [] };
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
    const initialState: GraphState = { nodes: [], edges: [] };
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
    const initialState: GraphState = { nodes: [], edges: [] };
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
    const initialState: GraphState = { nodes: [], edges: [] };
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

  it('keeps history actions stable so a graph-recording effect settles', () => {
    const { result } = renderHook(() => useHistory(0));
    const { push, undo, redo } = result.current;
    act(() => result.current.push(1));
    expect(result.current.push).toBe(push);
    expect(result.current.undo).toBe(undo);
    expect(result.current.redo).toBe(redo);
  });

  it('retains intermediate snapshots when several actions share a React batch', () => {
    const { result } = renderHook(() => useHistory(0));
    act(() => { result.current.push(1); result.current.push(2); });
    act(() => { expect(result.current.undo()).toBe(1); });
    act(() => { expect(result.current.undo()).toBe(0); });
    act(() => { expect(result.current.redo()).toBe(1); });
  });

  it('does not create undo history when recording the current snapshot', () => {
    const snapshot = { nodes: [], edges: [] };
    const { result } = renderHook(() => useHistory(snapshot));
    act(() => result.current.push(snapshot));
    expect(result.current.canUndo).toBe(false);
    act(() => { expect(result.current.undo()).toBeNull(); });
  });

  it('should clear history', () => {
    const initialState: GraphState = { nodes: [], edges: [] };
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
