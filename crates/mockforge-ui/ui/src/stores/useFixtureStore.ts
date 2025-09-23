import { create } from 'zustand';
import type { FixtureInfo, FixtureDiff, DiffChange } from '../types';
import * as Diff from 'diff';

interface FixtureStore {
  fixtures: FixtureInfo[];
  selectedFixture: FixtureInfo | null;
  diffHistory: FixtureDiff[];
  setFixtures: (fixtures: FixtureInfo[]) => void;
  selectFixture: (fixture: FixtureInfo) => void;
  updateFixture: (fixtureId: string, content: string) => void;
  renameFixture: (fixtureId: string, newName: string) => void;
  moveFixture: (fixtureId: string, newPath: string) => void;
  deleteFixture: (fixtureId: string) => void;
  addFixture: (fixture: FixtureInfo) => void;
  generateDiff: (fixtureId: string, newContent: string) => FixtureDiff;
  clearSelection: () => void;
}

// Mock fixtures data
const mockFixtures: FixtureInfo[] = [
  {
    id: 'user-get-list',
    name: 'users-list.json',
    path: 'http/get/users/users-list.json',
    content: `{
  "users": [
    {
      "id": 1,
      "name": "John Doe",
      "email": "john@example.com",
      "active": true
    },
    {
      "id": 2,
      "name": "Jane Smith", 
      "email": "jane@example.com",
      "active": true
    }
  ],
  "total": 2,
  "page": 1
}`,
    size_bytes: 234,
    last_modified: '2024-01-15T10:30:00Z',
    version: 1,
    route_path: '/api/users',
    method: 'GET',
  },
  {
    id: 'user-post-create',
    name: 'create-user.json',
    path: 'http/post/users/create-user.json',
    content: `{
  "id": 3,
  "name": "{{request.body.name}}",
  "email": "{{request.body.email}}",
  "active": true,
  "created_at": "{{now}}",
  "profile": {
    "bio": "New user profile",
    "avatar_url": null
  }
}`,
    size_bytes: 178,
    last_modified: '2024-01-14T15:45:00Z',
    version: 2,
    route_path: '/api/users',
    method: 'POST',
  },
  {
    id: 'order-get-details',
    name: 'order-details.json',
    path: 'http/get/orders/order-details.json',
    content: `{
  "id": "{{request.path.orderId}}",
  "customer_id": 1,
  "status": "pending",
  "items": [
    {
      "product_id": 101,
      "name": "Widget",
      "quantity": 2,
      "price": 19.99
    }
  ],
  "total": 39.98,
  "created_at": "{{now-1h}}"
}`,
    size_bytes: 298,
    last_modified: '2024-01-16T08:20:00Z',
    version: 1,
    route_path: '/api/orders/{orderId}',
    method: 'GET',
  },
  {
    id: 'grpc-inventory-item',
    name: 'inventory-item.json',
    path: 'grpc/inventory/inventory-item.json',
    content: `{
  "item": {
    "id": "{{uuid}}",
    "sku": "WIDGET-001",
    "name": "Premium Widget",
    "description": "High-quality widget for all your needs",
    "price": 29.99,
    "stock_quantity": 150,
    "category": "electronics",
    "attributes": {
      "color": "blue",
      "material": "plastic",
      "weight_kg": 0.5
    },
    "created_at": "{{now-30d}}",
    "updated_at": "{{now}}"
  }
}`,
    size_bytes: 445,
    last_modified: '2024-01-13T14:12:00Z',
    version: 1,
    route_path: 'inventory.InventoryService/GetItem',
  },
];

// Helper function to generate diff between two strings using Myers diff algorithm
const generateTextDiff = (oldContent: string, newContent: string): DiffChange[] => {
  const changes: DiffChange[] = [];
  const patches = Diff.diffLines(oldContent, newContent);
  let lineNumber = 1;

  for (const patch of patches) {
    const lines = patch.value.split('\n');
    // Remove the last empty line if it exists (diff adds it)
    if (lines[lines.length - 1] === '') {
      lines.pop();
    }

    for (const line of lines) {
      if (patch.added) {
        // Check if the last change was a remove at the same line - if so, make it a modify
        const lastChange = changes[changes.length - 1];
        if (lastChange && lastChange.type === 'remove' && lastChange.line_number === lineNumber) {
          lastChange.type = 'modify';
          lastChange.content = line;
          lastChange.old_content = lastChange.content;
          // Don't push new change
        } else {
          changes.push({
            type: 'add',
            line_number: lineNumber,
            content: line,
          });
        }
        lineNumber++;
      } else if (patch.removed) {
        changes.push({
          type: 'remove',
          line_number: lineNumber,
          content: line,
        });
        // Don't increment line number for removed lines
      } else {
        // Unchanged line
        lineNumber++;
      }
    }
  }

  return changes;
};

export const useFixtureStore = create<FixtureStore>((set, get) => ({
  fixtures: mockFixtures,
  selectedFixture: null,
  diffHistory: [],
  
  setFixtures: (fixtures) => set({ fixtures }),
  
  selectFixture: (fixture) => set({ selectedFixture: fixture }),
  
  clearSelection: () => set({ selectedFixture: null }),
  
  updateFixture: (fixtureId, content) => set((state) => {
    const fixture = state.fixtures.find(f => f.id === fixtureId);
    if (!fixture) return state;

    // Generate diff for history
    const diff = get().generateDiff(fixtureId, content);
    
    const updatedFixtures = state.fixtures.map(f => 
      f.id === fixtureId 
        ? { 
            ...f, 
            content, 
            version: f.version + 1,
            size_bytes: new Blob([content]).size,
            last_modified: new Date().toISOString(),
          }
        : f
    );

    const updatedSelected = state.selectedFixture?.id === fixtureId 
      ? updatedFixtures.find(f => f.id === fixtureId) || null
      : state.selectedFixture;

    return {
      fixtures: updatedFixtures,
      selectedFixture: updatedSelected,
      diffHistory: diff.changes.length > 0 ? [diff, ...state.diffHistory.slice(0, 9)] : state.diffHistory,
    };
  }),
  
  renameFixture: (fixtureId, newName) => set((state) => ({
    fixtures: state.fixtures.map(f => 
      f.id === fixtureId 
        ? { 
            ...f, 
            name: newName,
            last_modified: new Date().toISOString(),
          }
        : f
    ),
    selectedFixture: state.selectedFixture?.id === fixtureId 
      ? { ...state.selectedFixture, name: newName, last_modified: new Date().toISOString() }
      : state.selectedFixture,
  })),
  
  moveFixture: (fixtureId, newPath) => set((state) => ({
    fixtures: state.fixtures.map(f => 
      f.id === fixtureId 
        ? { 
            ...f, 
            path: newPath,
            last_modified: new Date().toISOString(),
          }
        : f
    ),
    selectedFixture: state.selectedFixture?.id === fixtureId 
      ? { ...state.selectedFixture, path: newPath, last_modified: new Date().toISOString() }
      : state.selectedFixture,
  })),
  
  deleteFixture: (fixtureId) => set((state) => ({
    fixtures: state.fixtures.filter(f => f.id !== fixtureId),
    selectedFixture: state.selectedFixture?.id === fixtureId ? null : state.selectedFixture,
  })),
  
  addFixture: (fixture) => set((state) => ({
    fixtures: [...state.fixtures, fixture],
  })),
  
  generateDiff: (fixtureId, newContent) => {
    const state = get();
    const fixture = state.fixtures.find(f => f.id === fixtureId);
    if (!fixture) {
      throw new Error(`Fixture with id ${fixtureId} not found`);
    }

    const changes = generateTextDiff(fixture.content, newContent);
    
    return {
      id: `${fixtureId}-${Date.now()}`,
      name: fixture.name,
      old_content: fixture.content,
      new_content: newContent,
      changes,
      timestamp: new Date().toISOString(),
    };
  },
}));