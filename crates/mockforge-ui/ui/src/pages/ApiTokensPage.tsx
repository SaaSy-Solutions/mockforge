import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Key,
  Plus,
  Trash2,
  Copy,
  CheckCircle2,
  Eye,
  EyeOff,
  Calendar,
  AlertTriangle,
} from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';

// Types
interface ApiToken {
  id: string;
  name: string;
  token_prefix: string;
  scopes: string[];
  expires_at?: string;
  last_used_at?: string;
  created_at: string;
}

interface CreateTokenRequest {
  name: string;
  scopes: string[];
  expires_days?: number;
}

interface CreateTokenResponse {
  token: string; // Full token (only shown once)
  token_info: ApiToken;
}

// API base URL
const API_BASE = '/api/v1';

async function fetchTokens(): Promise<ApiToken[]> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/tokens`, {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    throw new Error('Failed to fetch tokens');
  }
  return response.json();
}

async function createToken(request: CreateTokenRequest): Promise<CreateTokenResponse> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/tokens`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(request),
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || 'Failed to create token');
  }
  return response.json();
}

async function deleteToken(tokenId: string): Promise<void> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/tokens/${tokenId}`, {
    method: 'DELETE',
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    throw new Error('Failed to delete token');
  }
}

const AVAILABLE_SCOPES = [
  { value: 'read:packages', label: 'Read Packages', description: 'Read and search packages' },
  { value: 'publish:packages', label: 'Publish Packages', description: 'Publish new package versions' },
  { value: 'read:projects', label: 'Read Projects', description: 'Read project information' },
  { value: 'write:projects', label: 'Write Projects', description: 'Create and update projects' },
  { value: 'deploy:mocks', label: 'Deploy Mocks', description: 'Deploy hosted mock services' },
  { value: 'admin:org', label: 'Admin Organization', description: 'Full organization administration' },
];

export function ApiTokensPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
  const [newTokenName, setNewTokenName] = useState('');
  const [selectedScopes, setSelectedScopes] = useState<string[]>([]);
  const [expiresDays, setExpiresDays] = useState<number | undefined>(undefined);
  const [newToken, setNewToken] = useState<string | null>(null);
  const [showToken, setShowToken] = useState(false);

  // Fetch tokens
  const { data: tokens, isLoading } = useQuery({
    queryKey: ['api-tokens'],
    queryFn: fetchTokens,
  });

  // Create token mutation
  const createTokenMutation = useMutation({
    mutationFn: createToken,
    onSuccess: (data) => {
      setNewToken(data.token);
      queryClient.invalidateQueries({ queryKey: ['api-tokens'] });
      setIsCreateDialogOpen(false);
      // Reset form
      setNewTokenName('');
      setSelectedScopes([]);
      setExpiresDays(undefined);
    },
    onError: (error: Error) => {
      showToast({
        title: 'Error',
        description: error.message || 'Failed to create token',
        variant: 'destructive',
      });
    },
  });

  // Delete token mutation
  const deleteTokenMutation = useMutation({
    mutationFn: deleteToken,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-tokens'] });
      showToast({
        title: 'Success',
        description: 'Token deleted successfully',
      });
    },
    onError: (error: Error) => {
      showToast({
        title: 'Error',
        description: error.message || 'Failed to delete token',
        variant: 'destructive',
      });
    },
  });

  const handleCreateToken = () => {
    if (!newTokenName.trim()) {
      showToast({
        title: 'Error',
        description: 'Token name is required',
        variant: 'destructive',
      });
      return;
    }
    if (selectedScopes.length === 0) {
      showToast({
        title: 'Error',
        description: 'At least one scope is required',
        variant: 'destructive',
      });
      return;
    }
    createTokenMutation.mutate({
      name: newTokenName,
      scopes: selectedScopes,
      expires_days: expiresDays,
    });
  };

  const handleCopyToken = (token: string) => {
    navigator.clipboard.writeText(token);
    showToast({
      title: 'Copied',
      description: 'Token copied to clipboard',
    });
  };

  const toggleScope = (scope: string) => {
    setSelectedScopes((prev) =>
      prev.includes(scope) ? prev.filter((s) => s !== scope) : [...prev, scope]
    );
  };

  const formatDate = (dateString?: string) => {
    if (!dateString) return 'Never';
    return new Date(dateString).toLocaleDateString();
  };

  const isExpired = (expiresAt?: string) => {
    if (!expiresAt) return false;
    return new Date(expiresAt) < new Date();
  };

  return (
    <div className="container mx-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">API Tokens</h1>
          <p className="text-muted-foreground mt-2">
            Manage personal access tokens for CLI and API access
          </p>
        </div>
        <Button onClick={() => setIsCreateDialogOpen(true)}>
          <Plus className="w-4 h-4 mr-2" />
          Create Token
        </Button>
      </div>

      {/* New Token Display Dialog */}
      {newToken && (
        <Dialog open={!!newToken} onOpenChange={() => setNewToken(null)}>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Token Created</DialogTitle>
              <DialogDescription>
                Copy this token now. You won't be able to see it again!
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4">
              <div className="relative">
                <Input
                  type={showToken ? 'text' : 'password'}
                  value={newToken}
                  readOnly
                  className="font-mono text-sm"
                />
                <Button
                  variant="ghost"
                  size="sm"
                  className="absolute right-2 top-1/2 -translate-y-1/2"
                  onClick={() => setShowToken(!showToken)}
                >
                  {showToken ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </Button>
              </div>
              <div className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-3">
                <div className="flex items-start">
                  <AlertTriangle className="w-4 h-4 mr-2 text-yellow-600 dark:text-yellow-400 mt-0.5" />
                  <p className="text-sm text-yellow-800 dark:text-yellow-200">
                    Make sure to copy this token. It will not be shown again.
                  </p>
                </div>
              </div>
            </div>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => {
                  setNewToken(null);
                  setShowToken(false);
                }}
              >
                Close
              </Button>
              <Button onClick={() => handleCopyToken(newToken)}>
                <Copy className="w-4 h-4 mr-2" />
                Copy Token
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}

      {/* Create Token Dialog */}
      <Dialog open={isCreateDialogOpen} onOpenChange={setIsCreateDialogOpen}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Create API Token</DialogTitle>
            <DialogDescription>
              Create a personal access token for CLI and API access
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label htmlFor="token-name">Token Name</Label>
              <Input
                id="token-name"
                placeholder="e.g., CLI Development"
                value={newTokenName}
                onChange={(e) => setNewTokenName(e.target.value)}
              />
            </div>
            <div>
              <Label>Scopes</Label>
              <div className="mt-2 space-y-2 max-h-64 overflow-y-auto">
                {AVAILABLE_SCOPES.map((scope) => (
                  <div
                    key={scope.value}
                    className="flex items-start space-x-3 p-3 border rounded-lg hover:bg-accent cursor-pointer"
                    onClick={() => toggleScope(scope.value)}
                  >
                    <input
                      type="checkbox"
                      checked={selectedScopes.includes(scope.value)}
                      onChange={() => toggleScope(scope.value)}
                      className="mt-1"
                    />
                    <div className="flex-1">
                      <div className="font-medium">{scope.label}</div>
                      <div className="text-sm text-muted-foreground">{scope.description}</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
            <div>
              <Label htmlFor="expires-days">Expires In (Days)</Label>
              <Input
                id="expires-days"
                type="number"
                placeholder="Leave empty for no expiration"
                value={expiresDays || ''}
                onChange={(e) =>
                  setExpiresDays(e.target.value ? parseInt(e.target.value) : undefined)
                }
                min="1"
              />
              <p className="text-sm text-muted-foreground mt-1">
                Optional: Set expiration in days. Leave empty for no expiration.
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setIsCreateDialogOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleCreateToken}
              disabled={createTokenMutation.isPending || !newTokenName.trim() || selectedScopes.length === 0}
            >
              {createTokenMutation.isPending ? 'Creating...' : 'Create Token'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Tokens List */}
      {isLoading ? (
        <div className="text-center py-12">Loading tokens...</div>
      ) : tokens && tokens.length > 0 ? (
        <div className="space-y-4">
          {tokens.map((token) => (
            <Card key={token.id}>
              <CardContent className="p-6">
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center space-x-2 mb-2">
                      <h3 className="font-semibold">{token.name}</h3>
                      {isExpired(token.expires_at) && (
                        <Badge variant="destructive">Expired</Badge>
                      )}
                      {token.expires_at && !isExpired(token.expires_at) && (
                        <Badge variant="secondary">
                          Expires {formatDate(token.expires_at)}
                        </Badge>
                      )}
                    </div>
                    <div className="flex items-center space-x-4 text-sm text-muted-foreground mb-3">
                      <div className="flex items-center">
                        <Key className="w-4 h-4 mr-1" />
                        <span className="font-mono">{token.token_prefix}...</span>
                      </div>
                      {token.last_used_at && (
                        <div className="flex items-center">
                          <Calendar className="w-4 h-4 mr-1" />
                          Last used: {formatDate(token.last_used_at)}
                        </div>
                      )}
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {token.scopes.map((scope) => {
                        const scopeInfo = AVAILABLE_SCOPES.find((s) => s.value === scope);
                        return (
                          <Badge key={scope} variant="outline">
                            {scopeInfo?.label || scope}
                          </Badge>
                        );
                      })}
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      if (confirm('Are you sure you want to delete this token?')) {
                        deleteTokenMutation.mutate(token.id);
                      }
                    }}
                    disabled={deleteTokenMutation.isPending}
                  >
                    <Trash2 className="w-4 h-4 text-destructive" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <Card>
          <CardContent className="p-12 text-center">
            <Key className="w-12 h-12 mx-auto text-muted-foreground mb-4" />
            <h3 className="text-lg font-semibold mb-2">No API Tokens</h3>
            <p className="text-muted-foreground mb-4">
              Create your first API token to get started with CLI and API access
            </p>
            <Button onClick={() => setIsCreateDialogOpen(true)}>
              <Plus className="w-4 h-4 mr-2" />
              Create Token
            </Button>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
