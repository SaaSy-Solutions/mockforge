import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/Badge';
import {
  Users,
  UserPlus,
  Mail,
  Shield,
  TrendingUp,
  Settings,
  Trash2,
  Edit,
  CheckCircle,
  XCircle,
  Clock,
  BarChart3,
  UserCheck,
  UserX
} from 'lucide-react';
import { apiService } from '@/services/api';
import { useToast } from '@/components/ui/ToastProvider';

// Types
interface User {
  id: string;
  username: string;
  email: string;
  role: 'admin' | 'editor' | 'viewer';
  status: 'active' | 'inactive' | 'pending';
  display_name?: string;
  avatar_url?: string;
  created_at: string;
  last_activity?: string;
  team_id?: string;
}

interface Team {
  id: string;
  name: string;
  description?: string;
  owner_id: string;
  member_count: number;
  created_at: string;
  quota?: Quota;
}

interface Invitation {
  id: string;
  email: string;
  role: 'admin' | 'editor' | 'viewer';
  team_id?: string;
  status: 'pending' | 'accepted' | 'expired';
  expires_at: string;
  created_by: string;
  created_at: string;
}

interface Quota {
  max_users: number;
  max_teams: number;
  max_requests_per_month: number;
  max_storage_gb: number;
  current_users: number;
  current_teams: number;
  current_requests_this_month: number;
  current_storage_gb: number;
}

interface Analytics {
  total_users: number;
  active_users: number;
  new_users_this_month: number;
  total_teams: number;
  invitations_sent: number;
  invitations_accepted: number;
  user_activity: {
    date: string;
    active_users: number;
    requests: number;
  }[];
}

export function UserManagementPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [selectedTab, setSelectedTab] = useState('users');
  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteRole, setInviteRole] = useState<'admin' | 'editor' | 'viewer'>('viewer');
  const [inviteTeamId, setInviteTeamId] = useState<string>('');

  // Fetch users
  const { data: users, isLoading: usersLoading } = useQuery({
    queryKey: ['users'],
    queryFn: async () => {
      const response = await apiService.get('/api/users');
      return response.data as User[];
    },
  });

  // Fetch teams
  const { data: teams, isLoading: teamsLoading } = useQuery({
    queryKey: ['teams'],
    queryFn: async () => {
      const response = await apiService.get('/api/teams');
      return response.data as Team[];
    },
  });

  // Fetch invitations
  const { data: invitations, isLoading: invitationsLoading } = useQuery({
    queryKey: ['invitations'],
    queryFn: async () => {
      const response = await apiService.get('/api/invitations');
      return response.data as Invitation[];
    },
  });

  // Fetch quotas
  const { data: quota } = useQuery({
    queryKey: ['quota'],
    queryFn: async () => {
      const response = await apiService.get('/api/quota');
      return response.data as Quota;
    },
  });

  // Fetch analytics
  const { data: analytics } = useQuery({
    queryKey: ['user-analytics'],
    queryFn: async () => {
      const response = await apiService.get('/api/analytics/users');
      return response.data as Analytics;
    },
  });

  // Invite user mutation
  const inviteUser = useMutation({
    mutationFn: async (data: { email: string; role: string; team_id?: string }) => {
      const response = await apiService.post('/api/invitations', data);
      return response.data;
    },
    onSuccess: () => {
      showToast('success', 'Invitation sent', `Invitation sent to ${inviteEmail}`);
      setInviteEmail('');
      queryClient.invalidateQueries({ queryKey: ['invitations'] });
    },
    onError: (error: any) => {
      showToast('error', 'Failed to send invitation', error.response?.data?.message || 'An error occurred');
    },
  });

  // Update user role mutation
  const updateUserRole = useMutation({
    mutationFn: async ({ userId, role }: { userId: string; role: string }) => {
      const response = await apiService.put(`/api/users/${userId}/role`, { role });
      return response.data;
    },
    onSuccess: () => {
      showToast('success', 'User role updated');
      queryClient.invalidateQueries({ queryKey: ['users'] });
    },
  });

  // Delete user mutation
  const deleteUser = useMutation({
    mutationFn: async (userId: string) => {
      await apiService.delete(`/api/users/${userId}`);
    },
    onSuccess: () => {
      showToast('success', 'User deleted');
      queryClient.invalidateQueries({ queryKey: ['users'] });
    },
  });

  // Resend invitation mutation
  const resendInvitation = useMutation({
    mutationFn: async (invitationId: string) => {
      await apiService.post(`/api/invitations/${invitationId}/resend`);
    },
    onSuccess: () => {
      showToast('success', 'Invitation resent');
      queryClient.invalidateQueries({ queryKey: ['invitations'] });
    },
  });

  // Cancel invitation mutation
  const cancelInvitation = useMutation({
    mutationFn: async (invitationId: string) => {
      await apiService.delete(`/api/invitations/${invitationId}`);
    },
    onSuccess: () => {
      showToast('success', 'Invitation cancelled');
      queryClient.invalidateQueries({ queryKey: ['invitations'] });
    },
  });

  const handleInvite = () => {
    if (!inviteEmail) {
      showToast('error', 'Email required', 'Please enter an email address');
      return;
    }
    inviteUser.mutate({
      email: inviteEmail,
      role: inviteRole,
      team_id: inviteTeamId || undefined,
    });
  };

  const getRoleBadge = (role: string) => {
    const colors = {
      admin: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
      editor: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
      viewer: 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200',
    };
    return (
      <Badge className={colors[role as keyof typeof colors] || colors.viewer}>
        {role}
      </Badge>
    );
  };

  const getStatusBadge = (status: string) => {
    if (status === 'active') {
      return <Badge className="bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">
        <CheckCircle className="w-3 h-3 mr-1" />
        Active
      </Badge>;
    } else if (status === 'pending') {
      return <Badge className="bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200">
        <Clock className="w-3 h-3 mr-1" />
        Pending
      </Badge>;
    } else {
      return <Badge className="bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200">
        <XCircle className="w-3 h-3 mr-1" />
        Inactive
      </Badge>;
    }
  };

  return (
    <div className="container mx-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">User Management</h1>
          <p className="text-muted-foreground mt-1">
            Manage users, teams, invitations, and quotas
          </p>
        </div>
      </div>

      <Tabs value={selectedTab} onValueChange={setSelectedTab} className="space-y-4">
        <TabsList>
          <TabsTrigger value="users">
            <Users className="w-4 h-4 mr-2" />
            Users
          </TabsTrigger>
          <TabsTrigger value="teams">
            <Shield className="w-4 h-4 mr-2" />
            Teams
          </TabsTrigger>
          <TabsTrigger value="invitations">
            <Mail className="w-4 h-4 mr-2" />
            Invitations
          </TabsTrigger>
          <TabsTrigger value="quota">
            <Settings className="w-4 h-4 mr-2" />
            Quotas
          </TabsTrigger>
          <TabsTrigger value="analytics">
            <BarChart3 className="w-4 h-4 mr-2" />
            Analytics
          </TabsTrigger>
        </TabsList>

        {/* Users Tab */}
        <TabsContent value="users" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Users</CardTitle>
              <CardDescription>
                Manage user accounts and permissions
              </CardDescription>
            </CardHeader>
            <CardContent>
              {usersLoading ? (
                <div className="text-center py-8">Loading users...</div>
              ) : (
                <div className="space-y-4">
                  <div className="grid gap-4">
                    {users?.map((user) => (
                      <Card key={user.id}>
                        <CardContent className="p-4">
                          <div className="flex items-center justify-between">
                            <div className="flex items-center space-x-4">
                              <div className="w-10 h-10 bg-primary rounded-full flex items-center justify-center text-primary-foreground font-medium">
                                {user.display_name?.charAt(0) || user.username.charAt(0).toUpperCase()}
                              </div>
                              <div>
                                <div className="font-medium">
                                  {user.display_name || user.username}
                                </div>
                                <div className="text-sm text-muted-foreground">
                                  {user.email}
                                </div>
                                <div className="flex items-center space-x-2 mt-2">
                                  {getRoleBadge(user.role)}
                                  {getStatusBadge(user.status)}
                                </div>
                              </div>
                            </div>
                            <div className="flex items-center space-x-2">
                              <select
                                value={user.role}
                                onChange={(e) => updateUserRole.mutate({
                                  userId: user.id,
                                  role: e.target.value,
                                })}
                                className="px-3 py-1 border rounded-md text-sm"
                              >
                                <option value="viewer">Viewer</option>
                                <option value="editor">Editor</option>
                                <option value="admin">Admin</option>
                              </select>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => deleteUser.mutate(user.id)}
                              >
                                <Trash2 className="w-4 h-4" />
                              </Button>
                            </div>
                          </div>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Teams Tab */}
        <TabsContent value="teams" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Teams</CardTitle>
              <CardDescription>
                Manage team workspaces and members
              </CardDescription>
            </CardHeader>
            <CardContent>
              {teamsLoading ? (
                <div className="text-center py-8">Loading teams...</div>
              ) : (
                <div className="grid gap-4 md:grid-cols-2">
                  {teams?.map((team) => (
                    <Card key={team.id}>
                      <CardHeader>
                        <CardTitle>{team.name}</CardTitle>
                        <CardDescription>{team.description}</CardDescription>
                      </CardHeader>
                      <CardContent>
                        <div className="space-y-2">
                          <div className="flex items-center justify-between text-sm">
                            <span className="text-muted-foreground">Members:</span>
                            <span className="font-medium">{team.member_count}</span>
                          </div>
                          <div className="flex items-center justify-between text-sm">
                            <span className="text-muted-foreground">Created:</span>
                            <span className="font-medium">
                              {new Date(team.created_at).toLocaleDateString()}
                            </span>
                          </div>
                        </div>
                      </CardContent>
                    </Card>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Invitations Tab */}
        <TabsContent value="invitations" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Send Invitation</CardTitle>
              <CardDescription>
                Invite new users to join your workspace
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div>
                  <Label htmlFor="email">Email Address</Label>
                  <Input
                    id="email"
                    type="email"
                    placeholder="user@example.com"
                    value={inviteEmail}
                    onChange={(e) => setInviteEmail(e.target.value)}
                  />
                </div>
                <div>
                  <Label htmlFor="role">Role</Label>
                  <select
                    id="role"
                    value={inviteRole}
                    onChange={(e) => setInviteRole(e.target.value as any)}
                    className="w-full px-3 py-2 border rounded-md"
                  >
                    <option value="viewer">Viewer</option>
                    <option value="editor">Editor</option>
                    <option value="admin">Admin</option>
                  </select>
                </div>
                <div>
                  <Label htmlFor="team">Team (Optional)</Label>
                  <select
                    id="team"
                    value={inviteTeamId}
                    onChange={(e) => setInviteTeamId(e.target.value)}
                    className="w-full px-3 py-2 border rounded-md"
                  >
                    <option value="">None</option>
                    {teams?.map((team) => (
                      <option key={team.id} value={team.id}>{team.name}</option>
                    ))}
                  </select>
                </div>
                <Button onClick={handleInvite} disabled={inviteUser.isPending}>
                  <UserPlus className="w-4 h-4 mr-2" />
                  Send Invitation
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Pending Invitations</CardTitle>
              <CardDescription>
                Manage sent invitations
              </CardDescription>
            </CardHeader>
            <CardContent>
              {invitationsLoading ? (
                <div className="text-center py-8">Loading invitations...</div>
              ) : (
                <div className="space-y-4">
                  {invitations?.filter(i => i.status === 'pending').map((invitation) => (
                    <Card key={invitation.id}>
                      <CardContent className="p-4">
                        <div className="flex items-center justify-between">
                          <div>
                            <div className="font-medium">{invitation.email}</div>
                            <div className="text-sm text-muted-foreground">
                              Role: {invitation.role} • Expires: {new Date(invitation.expires_at).toLocaleDateString()}
                            </div>
                          </div>
                          <div className="flex items-center space-x-2">
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => resendInvitation.mutate(invitation.id)}
                            >
                              Resend
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => cancelInvitation.mutate(invitation.id)}
                            >
                              Cancel
                            </Button>
                          </div>
                        </div>
                      </CardContent>
                    </Card>
                  ))}
                  {invitations?.filter(i => i.status === 'pending').length === 0 && (
                    <div className="text-center py-8 text-muted-foreground">
                      No pending invitations
                    </div>
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Quota Tab */}
        <TabsContent value="quota" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Usage Quotas</CardTitle>
              <CardDescription>
                Monitor your current usage against limits
              </CardDescription>
            </CardHeader>
            <CardContent>
              {quota && (
                <div className="space-y-6">
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <Label>Users</Label>
                      <span className="text-sm text-muted-foreground">
                        {quota.current_users} / {quota.max_users}
                      </span>
                    </div>
                    <div className="w-full bg-gray-200 rounded-full h-2">
                      <div
                        className="bg-primary h-2 rounded-full"
                        style={{ width: `${(quota.current_users / quota.max_users) * 100}%` }}
                      />
                    </div>
                  </div>
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <Label>Teams</Label>
                      <span className="text-sm text-muted-foreground">
                        {quota.current_teams} / {quota.max_teams}
                      </span>
                    </div>
                    <div className="w-full bg-gray-200 rounded-full h-2">
                      <div
                        className="bg-primary h-2 rounded-full"
                        style={{ width: `${(quota.current_teams / quota.max_teams) * 100}%` }}
                      />
                    </div>
                  </div>
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <Label>Requests This Month</Label>
                      <span className="text-sm text-muted-foreground">
                        {quota.current_requests_this_month.toLocaleString()} / {quota.max_requests_per_month.toLocaleString()}
                      </span>
                    </div>
                    <div className="w-full bg-gray-200 rounded-full h-2">
                      <div
                        className="bg-primary h-2 rounded-full"
                        style={{ width: `${(quota.current_requests_this_month / quota.max_requests_per_month) * 100}%` }}
                      />
                    </div>
                  </div>
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <Label>Storage</Label>
                      <span className="text-sm text-muted-foreground">
                        {quota.current_storage_gb.toFixed(2)} GB / {quota.max_storage_gb} GB
                      </span>
                    </div>
                    <div className="w-full bg-gray-200 rounded-full h-2">
                      <div
                        className="bg-primary h-2 rounded-full"
                        style={{ width: `${(quota.current_storage_gb / quota.max_storage_gb) * 100}%` }}
                      />
                    </div>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Analytics Tab */}
        <TabsContent value="analytics" className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            <Card>
              <CardHeader className="pb-2">
                <CardDescription>Total Users</CardDescription>
                <CardTitle className="text-3xl">{analytics?.total_users || 0}</CardTitle>
              </CardHeader>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardDescription>Active Users</CardDescription>
                <CardTitle className="text-3xl">{analytics?.active_users || 0}</CardTitle>
              </CardHeader>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardDescription>New This Month</CardDescription>
                <CardTitle className="text-3xl">{analytics?.new_users_this_month || 0}</CardTitle>
              </CardHeader>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardDescription>Total Teams</CardDescription>
                <CardTitle className="text-3xl">{analytics?.total_teams || 0}</CardTitle>
              </CardHeader>
            </Card>
          </div>

          <Card>
            <CardHeader>
              <CardTitle>Invitation Statistics</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid gap-4 md:grid-cols-2">
                <div>
                  <div className="text-2xl font-bold">{analytics?.invitations_sent || 0}</div>
                  <div className="text-sm text-muted-foreground">Invitations Sent</div>
                </div>
                <div>
                  <div className="text-2xl font-bold">{analytics?.invitations_accepted || 0}</div>
                  <div className="text-sm text-muted-foreground">Invitations Accepted</div>
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
