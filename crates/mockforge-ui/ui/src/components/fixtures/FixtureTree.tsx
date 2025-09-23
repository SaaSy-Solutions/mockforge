import React, { useState } from 'react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import type { FixtureInfo } from '../../types';

interface FixtureTreeNode {
  id: string;
  name: string;
  type: 'file' | 'folder';
  children?: FixtureTreeNode[];
  fixture?: FixtureInfo;
  path: string;
}

interface FixtureTreeProps {
  fixtures: FixtureInfo[];
  onSelectFixture: (fixture: FixtureInfo) => void;
  onRenameFixture: (fixtureId: string, newName: string) => void;
  onMoveFixture: (fixtureId: string, newPath: string) => void;
  onDeleteFixture: (fixtureId: string) => void;
  selectedFixtureId?: string;
}

export function FixtureTree({ 
  fixtures, 
  onSelectFixture, 
  onRenameFixture, 
  onMoveFixture, 
  onDeleteFixture,
  selectedFixtureId 
}: FixtureTreeProps) {
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [newName, setNewName] = useState('');
  const [draggedItem, setDraggedItem] = useState<FixtureInfo | null>(null);

  // Build tree structure from flat fixture list
  const buildTree = (fixtures: FixtureInfo[]): FixtureTreeNode[] => {
    const tree: FixtureTreeNode[] = [];
    const folderMap = new Map<string, FixtureTreeNode>();

    fixtures.forEach(fixture => {
      const pathParts = fixture.path.split('/').filter(part => part);
      let currentPath = '';
      let currentLevel = tree;

      // Create folder structure
      for (let i = 0; i < pathParts.length - 1; i++) {
        const part = pathParts[i];
        currentPath = currentPath ? `${currentPath}/${part}` : part;
        
        let folder = folderMap.get(currentPath);
        if (!folder) {
          folder = {
            id: currentPath,
            name: part,
            type: 'folder',
            children: [],
            path: currentPath,
          };
          folderMap.set(currentPath, folder);
          currentLevel.push(folder);
        }
        
        currentLevel = folder.children!;
      }

      // Add the file
      const fileName = pathParts[pathParts.length - 1] || fixture.name;
      currentLevel.push({
        id: fixture.id,
        name: fileName,
        type: 'file',
        fixture,
        path: fixture.path,
      });
    });

    return tree;
  };

  const treeData = buildTree(fixtures);

  const toggleFolder = (folderId: string) => {
    const newExpanded = new Set(expandedFolders);
    if (newExpanded.has(folderId)) {
      newExpanded.delete(folderId);
    } else {
      newExpanded.add(folderId);
    }
    setExpandedFolders(newExpanded);
  };

  const startRename = (id: string, currentName: string) => {
    setRenamingId(id);
    setNewName(currentName);
  };

  const confirmRename = () => {
    if (renamingId && newName.trim()) {
      onRenameFixture(renamingId, newName.trim());
      setRenamingId(null);
      setNewName('');
    }
  };

  const cancelRename = () => {
    setRenamingId(null);
    setNewName('');
  };

  const handleDragStart = (fixture: FixtureInfo) => {
    setDraggedItem(fixture);
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
  };

  const handleDrop = (e: React.DragEvent, targetPath: string) => {
    e.preventDefault();
    if (draggedItem) {
      const newPath = `${targetPath}/${draggedItem.name}`;
      if (newPath !== draggedItem.path) {
        onMoveFixture(draggedItem.id, newPath);
      }
      setDraggedItem(null);
    }
  };

  const renderNode = (node: FixtureTreeNode, depth = 0) => {
    const isExpanded = expandedFolders.has(node.id);
    const isSelected = node.fixture?.id === selectedFixtureId;
    const isRenaming = renamingId === node.id;

    return (
      <div key={node.id}>
        <div
          className={`flex items-center space-x-2 px-2 py-1 hover:bg-accent rounded cursor-pointer ${
            isSelected ? 'bg-accent' : ''
          }`}
          style={{ paddingLeft: `${depth * 20 + 8}px` }}
          onClick={() => {
            if (node.type === 'folder') {
              toggleFolder(node.id);
            } else if (node.fixture) {
              onSelectFixture(node.fixture);
            }
          }}
          draggable={node.type === 'file'}
          onDragStart={() => node.fixture && handleDragStart(node.fixture)}
          onDragOver={handleDragOver}
          onDrop={(e) => node.type === 'folder' && handleDrop(e, node.path)}
        >
          {node.type === 'folder' && (
            <span className="text-muted-foreground">
              {isExpanded ? '📂' : '📁'}
            </span>
          )}
          {node.type === 'file' && (
            <span className="text-muted-foreground">📄</span>
          )}
          
          {isRenaming ? (
            <div className="flex items-center space-x-1 flex-1">
              <Input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') confirmRename();
                  if (e.key === 'Escape') cancelRename();
                }}
                className="h-6 text-xs"
                autoFocus
              />
              <Button size="sm" variant="ghost" onClick={confirmRename}>
                ✓
              </Button>
              <Button size="sm" variant="ghost" onClick={cancelRename}>
                ✗
              </Button>
            </div>
          ) : (
            <>
              <span className="flex-1 text-sm truncate">{node.name}</span>
              {node.fixture && (
                <div className="flex items-center space-x-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={(e) => {
                      e.stopPropagation();
                      startRename(node.id, node.name);
                    }}
                    className="h-6 w-6 p-0"
                  >
                    ✏️
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDeleteFixture(node.id);
                    }}
                    className="h-6 w-6 p-0"
                  >
                    🗑️
                  </Button>
                </div>
              )}
            </>
          )}
        </div>
        
        {node.type === 'folder' && isExpanded && node.children && (
          <div>
            {node.children.map(child => renderNode(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="border rounded-lg bg-card">
      <div className="p-3 border-b">
        <h3 className="font-semibold">Fixture Files</h3>
        <p className="text-xs text-muted-foreground mt-1">
          Drag files to move them between folders
        </p>
      </div>
      <div className="p-2 max-h-96 overflow-auto">
        {treeData.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground text-sm">
            No fixtures found
          </div>
        ) : (
          <div className="space-y-1">
            {treeData.map(node => renderNode(node))}
          </div>
        )}
      </div>
    </div>
  );
}