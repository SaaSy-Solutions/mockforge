//! History hook for undo/redo functionality
//!
//! Provides a React hook for managing undo/redo history with a configurable
//! capacity and state management.

import { useState, useCallback, useRef } from 'react';

interface UseHistoryOptions {
  capacity?: number;
}

interface UseHistoryReturn<T> {
  history: T;
  push: (state: T) => void;
  undo: () => T | null;
  redo: () => T | null;
  canUndo: boolean;
  canRedo: boolean;
  clear: () => void;
}

export function useHistory<T>(
  initialState: T,
  capacity: number = 50
): UseHistoryReturn<T> {
  const [currentState, setCurrentState] = useState<T>(initialState);
  const pastRef = useRef<T[]>([]);
  const futureRef = useRef<T[]>([]);

  const push = useCallback(
    (state: T) => {
      // Add current state to past
      pastRef.current.push(currentState);

      // Limit past history size
      if (pastRef.current.length > capacity) {
        pastRef.current.shift();
      }

      // Clear future when new state is pushed
      futureRef.current = [];

      setCurrentState(state);
    },
    [currentState, capacity]
  );

  const undo = useCallback(() => {
    if (pastRef.current.length === 0) {
      return null;
    }

    // Move current state to future
    futureRef.current.push(currentState);

    // Get previous state from past
    const previousState = pastRef.current.pop()!;
    setCurrentState(previousState);

    return previousState;
  }, [currentState]);

  const redo = useCallback(() => {
    if (futureRef.current.length === 0) {
      return null;
    }

    // Move current state to past
    pastRef.current.push(currentState);

    // Get next state from future
    const nextState = futureRef.current.pop()!;
    setCurrentState(nextState);

    return nextState;
  }, [currentState]);

  const clear = useCallback(() => {
    pastRef.current = [];
    futureRef.current = [];
    setCurrentState(initialState);
  }, [initialState]);

  return {
    history: currentState,
    push,
    undo,
    redo,
    canUndo: pastRef.current.length > 0,
    canRedo: futureRef.current.length > 0,
    clear,
  };
}
