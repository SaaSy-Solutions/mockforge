import React from 'react';
import { X, ExternalLink, Copy, Check } from 'lucide-react';
import { Button } from '../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/Card';
import { Badge } from '../ui/Badge';
import { StateIndicator } from './StateIndicator';
import type { GraphNode, GraphEdge } from '../../types/graph';

interface GraphDetailsPanelProps {
  selectedNode?: GraphNode | null;
  selectedEdge?: GraphEdge | null;
  onClose: () => void;
}

export function GraphDetailsPanel({
  selectedNode,
  selectedEdge,
  onClose,
}: GraphDetailsPanelProps) {
  const [copied, setCopied] = React.useState(false);

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  if (!selectedNode && !selectedEdge) {
    return null;
  }

  return (
    <div className="absolute right-0 top-0 bottom-0 w-96 bg-card border-l border-border shadow-xl z-10 overflow-y-auto">
      <div className="sticky top-0 bg-card border-b border-border p-4 flex items-center justify-between">
        <h3 className="text-lg font-semibold text-foreground">
          {selectedNode ? 'Node Details' : 'Edge Details'}
        </h3>
        <Button variant="ghost" size="sm" onClick={onClose}>
          <X className="h-4 w-4" />
        </Button>
      </div>

      <div className="p-4 space-y-4">
        {selectedNode && (
          <>
            <Card>
              <CardHeader>
                <CardTitle className="text-base">{selectedNode.label}</CardTitle>
                <CardDescription>Node Information</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <div>
                  <div className="text-xs font-medium text-muted-foreground mb-1">
                    ID
                  </div>
                  <div className="flex items-center gap-2">
                    <code className="text-sm font-mono bg-muted dark:bg-gray-700 px-2 py-1 rounded flex-1">
                      {selectedNode.id}
                    </code>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => copyToClipboard(selectedNode.id)}
                    >
                      {copied ? (
                        <Check className="h-4 w-4 text-success-500" />
                      ) : (
                        <Copy className="h-4 w-4" />
                      )}
                    </Button>
                  </div>
                </div>

                <div>
                  <div className="text-xs font-medium text-muted-foreground mb-1">
                    Type
                  </div>
                  <Badge variant="outline" className="capitalize">
                    {selectedNode.nodeType}
                  </Badge>
                </div>

                {selectedNode.protocol && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-1">
                      Protocol
                    </div>
                    <Badge variant="outline" className="uppercase">
                      {selectedNode.protocol}
                    </Badge>
                  </div>
                )}

                {selectedNode.currentState && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-1">
                      Current State
                    </div>
                    <StateIndicator state={selectedNode.currentState} />
                  </div>
                )}

                {Object.keys(selectedNode.metadata).length > 0 && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-2">
                      Metadata
                    </div>
                    <div className="bg-muted rounded-md p-3 space-y-2">
                      {Object.entries(selectedNode.metadata).map(([key, value]) => (
                        <div key={key} className="text-sm">
                          <span className="font-medium text-foreground">
                            {key}:
                          </span>{' '}
                          <span className="text-muted-foreground">
                            {typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value)}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </CardContent>
            </Card>
          </>
        )}

        {selectedEdge && (
          <>
            <Card>
              <CardHeader>
                <CardTitle className="text-base">Connection</CardTitle>
                <CardDescription>Edge Information</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <div>
                  <div className="text-xs font-medium text-muted-foreground mb-1">
                    From
                  </div>
                  <code className="text-sm font-mono bg-muted dark:bg-gray-700 px-2 py-1 rounded block">
                    {selectedEdge.from}
                  </code>
                </div>

                <div>
                  <div className="text-xs font-medium text-muted-foreground mb-1">
                    To
                  </div>
                  <code className="text-sm font-mono bg-muted dark:bg-gray-700 px-2 py-1 rounded block">
                    {selectedEdge.to}
                  </code>
                </div>

                <div>
                  <div className="text-xs font-medium text-muted-foreground mb-1">
                    Type
                  </div>
                  <Badge variant="outline" className="capitalize">
                    {selectedEdge.edgeType.replace(/([A-Z])/g, ' $1').trim()}
                  </Badge>
                </div>

                {selectedEdge.label && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-1">
                      Label
                    </div>
                    <div className="text-sm text-foreground">
                      {selectedEdge.label}
                    </div>
                  </div>
                )}

                {Object.keys(selectedEdge.metadata).length > 0 && (
                  <div>
                    <div className="text-xs font-medium text-muted-foreground mb-2">
                      Metadata
                    </div>
                    <div className="bg-muted rounded-md p-3 space-y-2">
                      {Object.entries(selectedEdge.metadata).map(([key, value]) => (
                        <div key={key} className="text-sm">
                          <span className="font-medium text-foreground">
                            {key}:
                          </span>{' '}
                          <span className="text-muted-foreground">
                            {typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value)}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </CardContent>
            </Card>
          </>
        )}
      </div>
    </div>
  );
}
