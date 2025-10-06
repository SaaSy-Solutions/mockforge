/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useFixtureStore } from '../useFixtureStore';

describe('useFixtureStore', () => {
  beforeEach(() => {
    // Reset store before each test
    const { result } = renderHook(() => useFixtureStore());
    act(() => {
      result.current.clearSelection();
      result.current.setFixtures([]);
    });
  });

  it('initializes with empty fixtures', () => {
    const { result } = renderHook(() => useFixtureStore());

    act(() => {
      result.current.setFixtures([]);
    });

    expect(result.current.fixtures).toEqual([]);
    expect(result.current.selectedFixture).toBeNull();
  });

  it('sets fixtures', () => {
    const { result } = renderHook(() => useFixtureStore());
    const mockFixtures = [
      {
        id: '1',
        name: 'test.json',
        path: 'http/get/test.json',
        content: '{"test": true}',
        size_bytes: 15,
        last_modified: '2024-01-01T00:00:00Z',
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z',
        version: '1',
        route_path: '/api/test',
        method: 'GET',
      },
    ];

    act(() => {
      result.current.setFixtures(mockFixtures);
    });

    expect(result.current.fixtures).toEqual(mockFixtures);
  });

  it('selects a fixture', () => {
    const { result } = renderHook(() => useFixtureStore());
    const mockFixture = {
      id: '1',
      name: 'test.json',
      path: 'http/get/test.json',
      content: '{"test": true}',
      size_bytes: 15,
      last_modified: '2024-01-01T00:00:00Z',
      createdAt: '2024-01-01T00:00:00Z',
      updatedAt: '2024-01-01T00:00:00Z',
      version: '1',
      route_path: '/api/test',
      method: 'GET',
    };

    act(() => {
      result.current.selectFixture(mockFixture);
    });

    expect(result.current.selectedFixture).toEqual(mockFixture);
  });

  it('clears selection', () => {
    const { result } = renderHook(() => useFixtureStore());
    const mockFixture = {
      id: '1',
      name: 'test.json',
      path: 'http/get/test.json',
      content: '{"test": true}',
      size_bytes: 15,
      last_modified: '2024-01-01T00:00:00Z',
      createdAt: '2024-01-01T00:00:00Z',
      updatedAt: '2024-01-01T00:00:00Z',
      version: '1',
      route_path: '/api/test',
      method: 'GET',
    };

    act(() => {
      result.current.selectFixture(mockFixture);
      result.current.clearSelection();
    });

    expect(result.current.selectedFixture).toBeNull();
  });

  it('updates fixture content', () => {
    const { result } = renderHook(() => useFixtureStore());
    const mockFixtures = [
      {
        id: '1',
        name: 'test.json',
        path: 'http/get/test.json',
        content: '{"test": true}',
        size_bytes: 15,
        last_modified: '2024-01-01T00:00:00Z',
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z',
        version: '1',
        route_path: '/api/test',
        method: 'GET',
      },
    ];

    act(() => {
      result.current.setFixtures(mockFixtures);
      result.current.updateFixture('1', '{"test": false}');
    });

    expect(result.current.fixtures[0].content).toBe('{"test": false}');
  });

  it('adds a new fixture', () => {
    const { result } = renderHook(() => useFixtureStore());
    const newFixture = {
      id: '2',
      name: 'new.json',
      path: 'http/post/new.json',
      content: '{"new": true}',
      size_bytes: 15,
      last_modified: '2024-01-01T00:00:00Z',
      createdAt: '2024-01-01T00:00:00Z',
      updatedAt: '2024-01-01T00:00:00Z',
      version: '1',
      route_path: '/api/new',
      method: 'POST',
    };

    act(() => {
      result.current.addFixture(newFixture);
    });

    expect(result.current.fixtures).toContainEqual(newFixture);
  });

  it('deletes a fixture', () => {
    const { result } = renderHook(() => useFixtureStore());
    const mockFixtures = [
      {
        id: '1',
        name: 'test.json',
        path: 'http/get/test.json',
        content: '{"test": true}',
        size_bytes: 15,
        last_modified: '2024-01-01T00:00:00Z',
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z',
        version: '1',
        route_path: '/api/test',
        method: 'GET',
      },
    ];

    act(() => {
      result.current.setFixtures(mockFixtures);
      result.current.deleteFixture('1');
    });

    expect(result.current.fixtures).toEqual([]);
  });

  it('generates diff for fixture changes', () => {
    const { result } = renderHook(() => useFixtureStore());

    const diff = result.current.generateDiff('1', '{"new": "content"}');

    expect(diff).toHaveProperty('fixtureId');
    expect(diff).toHaveProperty('changes');
    expect(diff).toHaveProperty('timestamp');
  });
});
