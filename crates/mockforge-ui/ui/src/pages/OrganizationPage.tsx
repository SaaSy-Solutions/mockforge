import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/Badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import {
  Building2,
  Users,
  UserPlus,
  Settings,
  Trash2,
  Edit,
  Crown,
  Shield,
  User,
  Mail,
} from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';

// Types - these would match your backend API
interface Organization {
  id: string;
  name: string;
  slug: string;
  plan: 'free' | 'pro' | 'team';
  owner_id: string;
  created_at: string;
}

interface OrgMember {
  id: string;
  user_id: string;
  username: string;
  email: string;
  role: 'owner' | 'admin' | 'member';
  created_at: string;
}

// API base URL - adjust based on your setup
const API_BASE = '/api/v1';

// Placeholder API functions - these would need to be implemented based on your backend
async function fetchOrganizations(): Promise<Organization[]> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/organizations`, {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    throw new Error('Failed to fetch organizations');
  }
  return response.json();
}

async function fetchOrgMembers(orgId: string): Promise<OrgMember[]> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch(`${API_BASE}/organizations/${orgId}/members`, {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
  });
  if (!response.ok) {
    throw new Error('Failed to fetch members');
  }
  return response.json();
}

export function OrganizationPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [selectedOrgId, setSelectedOrgId] = useState<string | null>(null);

  // Fetch organizations
  const { data: organizations, isLoading: orgsLoading } = useQuery({
    queryKey: ['organizations'],
    queryFn: fetchOrganizations,
  });

  // Fetch members for selected org
  const { data: members, isLoading: membersLoading } = useQuery({
    queryKey: ['org-members', selectedOrgId],
    queryFn: () => fetchOrgMembers(selectedOrgId!),
    enabled: !!selectedOrgId,
  });

  const getRoleIcon = (role: string) => {
    switch (role) {
      case 'owner':
        return <Crown className="w-4 h-4 text-yellow-500" />;
      case 'admin':
        return <Shield className="w-4 h-4 text-blue-500" />;
      default:
        return <User className="w-4 h-4 text-gray-500" />;
    }
  };

  const getRoleBadge = (role: string) => {
    switch (role) {
      case 'owner':
        return <Badge className="bg-yellow-500">Owner</Badge>;
      case 'admin':
        return <Badge className="bg-blue-500">Admin</Badge>;
      default:
        return <Badge variant="secondary">Member</Badge>;
    }
  };

  const getPlanBadge = (plan: string) => {
    switch (plan) {
      case 'team':
        return <Badge className="bg-purple-500">Team</Badge>;
      case 'pro':
        return <Badge className="bg-blue-500">Pro</Badge>;
      default:
        return <Badge variant="secondary">Free</Badge>;
    }
  };

  if (orgsLoading) {
    return (
      <div className="container mx-auto p-6">
        <div className="text-center py-12">Loading organizations...</div>
      </div>
    );
  }

  const selectedOrg = organizations?.find((org) => org.id === selectedOrgId);

  return (
    <div className="container mx-auto p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Organizations</h1>
        <p className="text-muted-foreground mt-2">
          Manage your organizations and team members
        </p>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        {/* Organizations List */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center">
              <Building2 className="w-5 h-5 mr-2" />
              Your Organizations
            </CardTitle>
            <CardDescription>Select an organization to manage</CardDescription>
          </CardHeader>
          <CardContent>
            {organizations && organizations.length > 0 ? (
              <div className="space-y-2">
                {organizations.map((org) => (
                  <div
                    key={org.id}
                    className={`p-4 border rounded-lg cursor-pointer transition-colors ${
                      selectedOrgId === org.id
                        ? 'border-primary bg-primary/5'
                        : 'hover:bg-accent'
                    }`}
                    onClick={() => setSelectedOrgId(org.id)}
                  >
                    <div className="flex items-center justify-between">
                      <div>
                        <div className="font-semibold">{org.name}</div>
                        <div className="text-sm text-muted-foreground">@{org.slug}</div>
                      </div>
                      {getPlanBadge(org.plan)}
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-8 text-muted-foreground">
                No organizations found
              </div>
            )}
          </CardContent>
        </Card>

        {/* Organization Details */}
        {selectedOrg ? (
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                <span>{selectedOrg.name}</span>
                {getPlanBadge(selectedOrg.plan)}
              </CardTitle>
              <CardDescription>@{selectedOrg.slug}</CardDescription>
            </CardHeader>
            <CardContent>
              <Tabs defaultValue="members" className="w-full">
                <TabsList className="grid w-full grid-cols-2">
                  <TabsTrigger value="members">Members</TabsTrigger>
                  <TabsTrigger value="settings">Settings</TabsTrigger>
                </TabsList>

                <TabsContent value="members" className="space-y-4 mt-4">
                  {membersLoading ? (
                    <div className="text-center py-4">Loading members...</div>
                  ) : members && members.length > 0 ? (
                    <div className="space-y-2">
                      {members.map((member) => (
                        <div
                          key={member.id}
                          className="flex items-center justify-between p-3 border rounded-lg"
                        >
                          <div className="flex items-center space-x-3">
                            {getRoleIcon(member.role)}
                            <div>
                              <div className="font-medium">{member.username}</div>
                              <div className="text-sm text-muted-foreground">{member.email}</div>
                            </div>
                          </div>
                          {getRoleBadge(member.role)}
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="text-center py-8 text-muted-foreground">
                      No members found
                    </div>
                  )}
                </TabsContent>

                <TabsContent value="settings" className="space-y-4 mt-4">
                  <div className="space-y-4">
                    <div>
                      <Label>Organization Name</Label>
                      <Input value={selectedOrg.name} disabled />
                    </div>
                    <div>
                      <Label>Slug</Label>
                      <Input value={selectedOrg.slug} disabled />
                    </div>
                    <div>
                      <Label>Plan</Label>
                      <div className="mt-2">{getPlanBadge(selectedOrg.plan)}</div>
                    </div>
                    <div>
                      <Label>Created</Label>
                      <div className="text-sm text-muted-foreground mt-1">
                        {new Date(selectedOrg.created_at).toLocaleDateString()}
                      </div>
                    </div>
                  </div>
                </TabsContent>
              </Tabs>
            </CardContent>
          </Card>
        ) : (
          <Card>
            <CardContent className="p-12 text-center">
              <Building2 className="w-12 h-12 mx-auto text-muted-foreground mb-4" />
              <h3 className="text-lg font-semibold mb-2">Select an Organization</h3>
              <p className="text-muted-foreground">
                Choose an organization from the list to view details and manage members
              </p>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}
