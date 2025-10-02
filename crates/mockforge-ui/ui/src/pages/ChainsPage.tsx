import React, { useEffect, useState } from 'react';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../components/ui/Table';
import { apiService } from '../services/api';
import type { ChainSummary } from '../types/chains';

interface ChainsPageProps {
  className?: string;
}

export const ChainsPage: React.FC<ChainsPageProps> = ({ className }) => {
  const [chains, setChains] = useState<ChainSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchChains = async () => {
      try {
        setLoading(true);
        const response = await apiService.listChains();
        setChains(response.chains);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load chains');
      } finally {
        setLoading(false);
      }
    };

    fetchChains();
  }, []);

  const handleDeleteChain = async (chainId: string) => {
    if (!window.confirm(`Are you sure you want to delete chain "${chainId}"?`)) {
      return;
    }

    try {
      await apiService.deleteChain(chainId);
      setChains(chains.filter(chain => chain.id !== chainId));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete chain');
    }
  };

  if (loading) {
    return (
      <div className={`p-6 ${className}`}>
        <div className="flex items-center justify-center h-64">
          <div className="text-lg">Loading chains...</div>
        </div>
      </div>
    );
  }

  return (
    <div className={`p-6 ${className}`}>
      <div className="flex justify-between items-center mb-6">
        <div>
          <h1 className="text-2xl font-bold">Request Chains</h1>
          <p className="text-muted-foreground">
            Manage and execute request chains for complex API workflows
          </p>
        </div>
        <Button>
          <PlusIcon className="h-4 w-4 mr-2" />
          Create Chain
        </Button>
      </div>

      {error && (
        <div className="mb-6 p-4 bg-destructive/10 border border-destructive/20 rounded-md">
          <p className="text-destructive">{error}</p>
        </div>
      )}

      <div className="grid gap-4">
        {chains.length === 0 ? (
          <Card>
            <CardContent className="flex flex-col items-center justify-center h-64">
              <div className="text-center">
                <h3 className="text-lg font-medium mb-2">No Chains Found</h3>
                <p className="text-muted-foreground mb-4">
                  Create your first request chain to get started with complex API workflow testing.
                </p>
                <Button variant="outline">
                  <PlusIcon className="h-4 w-4 mr-2" />
                  Create First Chain
                </Button>
              </div>
            </CardContent>
          </Card>
        ) : (
          <Card>
            <CardHeader>
              <CardTitle>Available Chains ({chains.length})</CardTitle>
              <CardDescription>
                Click on a chain to view details and execute it
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Description</TableHead>
                    <TableHead>Links</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Tags</TableHead>
                    <TableHead className="w-48">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {chains.map((chain) => (
                    <TableRow key={chain.id}>
                      <TableCell className="font-medium">{chain.name}</TableCell>
                      <TableCell className="max-w-md truncate">
                        {chain.description || 'No description'}
                      </TableCell>
                      <TableCell>{chain.linkCount}</TableCell>
                      <TableCell>
                        <Badge variant={chain.enabled ? 'default' : 'secondary'}>
                          {chain.enabled ? 'Enabled' : 'Disabled'}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <div className="flex gap-1">
                          {chain.tags?.map((tag) => (
                            <Badge key={tag} variant="outline" className="text-xs">
                              {tag}
                            </Badge>
                          ))}
                          {!chain.tags?.length && <span className="text-muted-foreground">—</span>}
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex gap-2">
                          <Button variant="outline" size="sm">
                            <EyeIcon className="h-4 w-4 mr-1" />
                            View
                          </Button>
                          <Button variant="outline" size="sm">
                            <PlayIcon className="h-4 w-4 mr-1" />
                            Execute
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleDeleteChain(chain.id)}
                          >
                            <TrashIcon className="h-4 w-4 mr-1" />
                            Delete
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
};

// Placeholder icons - should be replaced with proper icon components
const PlusIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
  </svg>
);

const EyeIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
  </svg>
);

const PlayIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M14.828 14.828a4 4 0 01-5.656 0M9 10h1m3 0h1m0 3a3 3 0 01-3-3m0 8a9 9 0 01-9-9M3 21l3-3m3 3l3-3m3 3l3-3M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
  </svg>
);

const TrashIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
  </svg>
);
