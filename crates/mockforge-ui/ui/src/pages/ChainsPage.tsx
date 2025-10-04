import React, { useEffect, useState } from 'react';
import { Plus, Eye, Play, Trash2, Loader2 } from 'lucide-react';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/Card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../components/ui/Table';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '../components/ui/Dialog';
import { Label } from '../components/ui/label';
import { Textarea } from '../components/ui/textarea';
import { apiService } from '../services/api';
import type { ChainSummary, ChainDefinition } from '../types/chains';

interface ChainsPageProps {
  className?: string;
}

export const ChainsPage: React.FC<ChainsPageProps> = ({ className }) => {
  const [chains, setChains] = useState<ChainSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  // Dialog states
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [chainToDelete, setChainToDelete] = useState<ChainSummary | null>(null);
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [viewDialogOpen, setViewDialogOpen] = useState(false);
  const [executeDialogOpen, setExecuteDialogOpen] = useState(false);
  const [selectedChain, setSelectedChain] = useState<ChainSummary | null>(null);
  const [executionResult, setExecutionResult] = useState<string | null>(null);
  const [executing, setExecuting] = useState(false);

  useEffect(() => {
    fetchChains();
  }, []);

  const fetchChains = async () => {
    try {
      setLoading(true);
      const response = await apiService.listChains();
      setChains(response.chains);
      setError(null);
    } catch (err) {
      console.error('Failed to fetch chains:', err);
      const errorMessage = err instanceof Error
        ? err.message.includes('not valid JSON') || err.message.includes('DOCTYPE')
          ? 'Chain API is not available. The backend may not be running with chain support enabled.'
          : err.message
        : 'Failed to load chains';
      setError(errorMessage);
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteChain = async () => {
    if (!chainToDelete) return;

    try {
      setDeletingId(chainToDelete.id);
      await apiService.deleteChain(chainToDelete.id);
      setChains(chains.filter(chain => chain.id !== chainToDelete.id));
      setDeleteDialogOpen(false);
      setChainToDelete(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete chain');
    } finally {
      setDeletingId(null);
    }
  };

  const handleCreateChain = () => {
    setCreateDialogOpen(true);
  };

  const [chainDetails, setChainDetails] = useState<ChainDefinition | null>(null);
  const [loadingDetails, setLoadingDetails] = useState(false);

  const handleViewChain = async (chain: ChainSummary) => {
    setSelectedChain(chain);
    setViewDialogOpen(true);
    setLoadingDetails(true);
    setChainDetails(null);

    try {
      const details = await apiService.getChain(chain.id);
      setChainDetails(details);
    } catch (err) {
      console.error('Failed to fetch chain details:', err);
    } finally {
      setLoadingDetails(false);
    }
  };

  const handleExecuteChain = async (chain: ChainSummary) => {
    setSelectedChain(chain);
    setExecuteDialogOpen(true);
    setExecuting(true);
    setExecutionResult(null);

    try {
      const result = await apiService.executeChain(chain.id);
      setExecutionResult(JSON.stringify(result, null, 2));
    } catch (err) {
      setExecutionResult(`Error: ${err instanceof Error ? err.message : 'Failed to execute chain'}`);
    } finally {
      setExecuting(false);
    }
  };

  const openDeleteDialog = (chain: ChainSummary) => {
    setChainToDelete(chain);
    setDeleteDialogOpen(true);
  };

  if (loading) {
    return (
      <div className={`p-6 ${className}`}>
        <div className="flex items-center justify-center h-64">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          <span className="ml-2 text-lg">Loading chains...</span>
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
        <Button onClick={handleCreateChain}>
          <Plus className="h-4 w-4 mr-2" />
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
                <Button variant="outline" onClick={handleCreateChain}>
                  <Plus className="h-4 w-4 mr-2" />
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
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleViewChain(chain)}
                          >
                            <Eye className="h-4 w-4 mr-1" />
                            View
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleExecuteChain(chain)}
                            disabled={!chain.enabled}
                          >
                            <Play className="h-4 w-4 mr-1" />
                            Execute
                          </Button>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => openDeleteDialog(chain)}
                            disabled={deletingId === chain.id}
                          >
                            {deletingId === chain.id ? (
                              <Loader2 className="h-4 w-4 mr-1 animate-spin" />
                            ) : (
                              <Trash2 className="h-4 w-4 mr-1" />
                            )}
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

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Chain</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete the chain "{chainToDelete?.name}"? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDeleteDialogOpen(false)}
              disabled={deletingId !== null}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleDeleteChain}
              disabled={deletingId !== null}
            >
              {deletingId !== null ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Deleting...
                </>
              ) : (
                'Delete'
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Create Chain Dialog */}
      <Dialog open={createDialogOpen} onOpenChange={setCreateDialogOpen}>
        <DialogContent className="max-w-4xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Create Chain</DialogTitle>
            <DialogDescription>
              Create a new request chain using YAML definition.
            </DialogDescription>
          </DialogHeader>
          <ChainCreationForm
            onClose={() => setCreateDialogOpen(false)}
            onSuccess={(newChain) => {
              setChains([...chains, newChain]);
              setCreateDialogOpen(false);
            }}
          />
        </DialogContent>
      </Dialog>

      {/* View Chain Dialog */}
      <Dialog open={viewDialogOpen} onOpenChange={setViewDialogOpen}>
        <DialogContent className="max-w-5xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{selectedChain?.name}</DialogTitle>
            <DialogDescription>
              {selectedChain?.description || 'No description provided'}
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            {loadingDetails ? (
              <div className="flex items-center justify-center h-32">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                <span className="ml-2">Loading chain details...</span>
              </div>
            ) : chainDetails ? (
              <div className="space-y-6">
                {/* Summary */}
                <div>
                  <h4 className="font-medium mb-3">Overview</h4>
                  <div className="grid grid-cols-3 gap-4 text-sm">
                    <div className="space-y-1">
                      <span className="text-muted-foreground">Status</span>
                      <div>
                        <Badge variant={chainDetails.config.enabled ? 'default' : 'secondary'}>
                          {chainDetails.config.enabled ? 'Enabled' : 'Disabled'}
                        </Badge>
                      </div>
                    </div>
                    <div className="space-y-1">
                      <span className="text-muted-foreground">Links</span>
                      <div className="font-medium">{chainDetails.links.length}</div>
                    </div>
                    <div className="space-y-1">
                      <span className="text-muted-foreground">Max Length</span>
                      <div className="font-medium">{chainDetails.config.maxChainLength}</div>
                    </div>
                  </div>
                  {chainDetails.tags && chainDetails.tags.length > 0 && (
                    <div className="mt-3">
                      <span className="text-sm text-muted-foreground">Tags: </span>
                      {chainDetails.tags.map((tag) => (
                        <Badge key={tag} variant="outline" className="ml-1 text-xs">
                          {tag}
                        </Badge>
                      ))}
                    </div>
                  )}
                </div>

                {/* Configuration */}
                <div>
                  <h4 className="font-medium mb-3">Configuration</h4>
                  <div className="bg-muted/50 rounded-lg p-4 space-y-2 text-sm">
                    <div className="grid grid-cols-2 gap-4">
                      <div>
                        <span className="text-muted-foreground">Global Timeout:</span>{' '}
                        <span className="font-medium">{chainDetails.config.globalTimeoutSecs}s</span>
                      </div>
                      <div>
                        <span className="text-muted-foreground">Parallel Execution:</span>{' '}
                        <span className="font-medium">
                          {chainDetails.config.enableParallelExecution ? 'Enabled' : 'Disabled'}
                        </span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Variables */}
                {chainDetails.variables && Object.keys(chainDetails.variables).length > 0 && (
                  <div>
                    <h4 className="font-medium mb-3">Variables</h4>
                    <div className="bg-muted/50 rounded-lg p-4">
                      <div className="space-y-2 text-sm font-mono">
                        {Object.entries(chainDetails.variables).map(([key, value]) => (
                          <div key={key} className="flex">
                            <span className="text-blue-600 dark:text-blue-400">{key}:</span>
                            <span className="ml-2 text-muted-foreground">
                              {typeof value === 'string' ? value : JSON.stringify(value) as string}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                )}

                {/* Links */}
                <div>
                  <h4 className="font-medium mb-3">Request Links ({chainDetails.links.length})</h4>
                  <div className="space-y-4">
                    {chainDetails.links.map((link, index) => (
                      <Card key={link.request.id} className="overflow-hidden">
                        <CardHeader className="pb-3">
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                              <span className="flex items-center justify-center w-6 h-6 rounded-full bg-primary/10 text-primary text-xs font-medium">
                                {index + 1}
                              </span>
                              <CardTitle className="text-base">{link.request.id}</CardTitle>
                            </div>
                            <Badge variant="outline" className="font-mono text-xs">
                              {link.request.method}
                            </Badge>
                          </div>
                        </CardHeader>
                        <CardContent className="space-y-3 text-sm">
                          <div>
                            <span className="text-muted-foreground">URL:</span>
                            <div className="font-mono text-xs bg-muted/50 p-2 rounded mt-1 break-all">
                              {link.request.url}
                            </div>
                          </div>

                          {link.request.headers && Object.keys(link.request.headers).length > 0 && (
                            <div>
                              <span className="text-muted-foreground">Headers:</span>
                              <div className="font-mono text-xs bg-muted/50 p-2 rounded mt-1 space-y-1">
                                {Object.entries(link.request.headers).map(([key, value]) => (
                                  <div key={key}>
                                    <span className="text-blue-600 dark:text-blue-400">{key}:</span>{' '}
                                    <span className="text-muted-foreground">{value}</span>
                                  </div>
                                ))}
                              </div>
                            </div>
                          )}

                          {link.request.body != null && (
                            <div>
                              <span className="text-muted-foreground">Body:</span>
                              <pre className="font-mono text-xs bg-muted/50 p-2 rounded mt-1 overflow-x-auto">
                                {JSON.stringify(link.request.body, null, 2) as string}
                              </pre>
                            </div>
                          )}

                          {link.extract && Object.keys(link.extract).length > 0 && (
                            <div>
                              <span className="text-muted-foreground">Extract Variables:</span>
                              <div className="font-mono text-xs bg-muted/50 p-2 rounded mt-1 space-y-1">
                                {Object.entries(link.extract).map(([key, value]) => {
                                  const stringValue = typeof value === 'string' ? value : String(value);
                                  return (
                                    <div key={key}>
                                      <span className="text-green-600 dark:text-green-400">{key}</span> ←{' '}
                                      <span className="text-muted-foreground">{stringValue}</span>
                                    </div>
                                  );
                                })}
                              </div>
                            </div>
                          )}

                          <div className="flex gap-4 text-xs">
                            {link.storeAs && (
                              <div>
                                <span className="text-muted-foreground">Store As:</span>{' '}
                                <span className="font-medium">{link.storeAs}</span>
                              </div>
                            )}
                            {link.dependsOn && link.dependsOn.length > 0 && (
                              <div>
                                <span className="text-muted-foreground">Depends On:</span>{' '}
                                <span className="font-medium">{link.dependsOn.join(', ')}</span>
                              </div>
                            )}
                          </div>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                </div>
              </div>
            ) : (
              <div className="text-center text-muted-foreground py-8">
                Failed to load chain details
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setViewDialogOpen(false)}>
              Close
            </Button>
            <Button
              onClick={() => {
                setViewDialogOpen(false);
                if (selectedChain) handleExecuteChain(selectedChain);
              }}
              disabled={!chainDetails || !chainDetails.config.enabled}
            >
              <Play className="h-4 w-4 mr-2" />
              Execute Chain
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Execute Chain Dialog */}
      <Dialog open={executeDialogOpen} onOpenChange={setExecuteDialogOpen}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>Execute Chain: {selectedChain?.name}</DialogTitle>
            <DialogDescription>
              {executing ? 'Executing chain...' : 'Chain execution results'}
            </DialogDescription>
          </DialogHeader>
          <div className="py-4">
            {executing ? (
              <div className="flex items-center justify-center h-32">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                <span className="ml-2">Executing chain...</span>
              </div>
            ) : (
              <div className="space-y-4">
                <div>
                  <h4 className="font-medium mb-2">Execution Result</h4>
                  <pre className="bg-muted p-4 rounded-md text-xs overflow-auto max-h-96">
                    {executionResult || 'No result available'}
                  </pre>
                </div>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setExecuteDialogOpen(false);
                setExecutionResult(null);
              }}
              disabled={executing}
            >
              Close
            </Button>
            {!executing && (
              <Button onClick={() => selectedChain && handleExecuteChain(selectedChain)}>
                <Play className="h-4 w-4 mr-2" />
                Execute Again
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

// Chain Creation Form Component
interface ChainCreationFormProps {
  onClose: () => void;
  onSuccess: (chain: ChainSummary) => void;
}

const ChainCreationForm: React.FC<ChainCreationFormProps> = ({ onClose, onSuccess }) => {
  const [yamlDefinition, setYamlDefinition] = useState(getDefaultYaml());
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCreate = async () => {
    try {
      setCreating(true);
      setError(null);

      const response = await apiService.createChain(yamlDefinition);

      // Fetch the newly created chain to get full details
      const chains = await apiService.listChains();
      const newChain = chains.chains.find(c => c.id === response.id);

      if (newChain) {
        onSuccess(newChain);
      } else {
        // If we can't find it, create a basic summary
        onSuccess({
          id: response.id,
          name: response.id,
          description: '',
          tags: [],
          enabled: true,
          linkCount: 0,
        });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create chain');
    } finally {
      setCreating(false);
    }
  };

  const loadExample = () => {
    setYamlDefinition(getExampleYaml());
  };

  return (
    <div className="space-y-4">
      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      <div className="space-y-2">
        <div className="flex justify-between items-center">
          <Label htmlFor="yaml-definition">YAML Definition</Label>
          <Button variant="outline" size="sm" onClick={loadExample}>
            Load Example
          </Button>
        </div>
        <Textarea
          id="yaml-definition"
          value={yamlDefinition}
          onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setYamlDefinition(e.target.value)}
          placeholder="Enter YAML chain definition..."
          className="font-mono text-sm min-h-[400px]"
        />
        <p className="text-xs text-muted-foreground">
          Define your chain using YAML format. Include id, name, description, config, links, variables, and tags.
        </p>
      </div>

      <DialogFooter>
        <Button variant="outline" onClick={onClose} disabled={creating}>
          Cancel
        </Button>
        <Button onClick={handleCreate} disabled={creating}>
          {creating ? (
            <>
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              Creating...
            </>
          ) : (
            'Create Chain'
          )}
        </Button>
      </DialogFooter>
    </div>
  );
};

function getDefaultYaml(): string {
  return `# Chain Definition
id: my-chain
name: My Request Chain
description: A simple request chain

config:
  enabled: true
  maxChainLength: 10
  globalTimeoutSecs: 60
  enableParallelExecution: false

links:
  - request:
      id: step1
      method: GET
      url: https://api.example.com/data
      headers:
        Content-Type: application/json
    storeAs: step1_response
    dependsOn: []

variables:
  base_url: https://api.example.com

tags:
  - example
`;
}

function getExampleYaml(): string {
  return `# Example: User Management Workflow
id: user-workflow-chain
name: User Management Workflow
description: |
  A complete user management workflow that demonstrates request chaining:
  1. Login to get authentication token
  2. Create a new user profile
  3. Update user settings
  4. Verify the user was created

config:
  enabled: true
  maxChainLength: 10
  globalTimeoutSecs: 60
  enableParallelExecution: false

links:
  # Step 1: Authentication - Login to get access token
  - request:
      id: login
      method: POST
      url: https://api.example.com/auth/login
      headers:
        Content-Type: application/json
      body:
        email: "user@example.com"
        password: "secure-password"
    extract:
      token: body.access_token
    storeAs: auth_response
    dependsOn: []

  # Step 2: Create user profile
  - request:
      id: create_user
      method: POST
      url: https://api.example.com/users
      headers:
        Content-Type: application/json
        Authorization: "Bearer {{chain.auth_response.body.access_token}}"
      body:
        name: "John Doe"
        email: "{{chain.auth_response.body.email}}"
        department: "Engineering"
    extract:
      user_id: body.id
      user_name: body.name
    storeAs: user_create_response
    dependsOn:
      - login

  # Step 3: Update user preferences
  - request:
      id: update_preferences
      method: PUT
      url: https://api.example.com/users/{{chain.user_create_response.body.id}}/preferences
      headers:
        Content-Type: application/json
        Authorization: "Bearer {{chain.auth_response.body.access_token}}"
      body:
        theme: dark
        notifications: true
        language: en
    storeAs: preferences_update
    dependsOn:
      - create_user

  # Step 4: Verify user creation
  - request:
      id: verify_user
      method: GET
      url: https://api.example.com/users/{{chain.user_create_response.body.id}}
      headers:
        Authorization: "Bearer {{chain.auth_response.body.access_token}}"
    storeAs: user_verification
    expectedStatus: [200]
    dependsOn:
      - create_user

variables:
  base_url: https://api.example.com
  api_version: v1

tags:
  - authentication
  - user-management
  - workflow
`;
}
