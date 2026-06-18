import React, { useState, useEffect } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/Badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/Dialog';
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
  Plus,
  Copy,
  FileText,
  ScrollText,
  Lock,
  BarChart3,
  Save,
  X,
  ChevronLeft,
  ChevronRight,
  ToggleLeft,
  ToggleRight,
  Brain,
  Link,
  AlertTriangle,
  CheckCircle2,
} from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';

// ─── Types ───────────────────────────────────────────────────────────────────

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
  avatar_url?: string;
  created_at: string;
}

interface AuditLogEntry {
  id: string;
  org_id: string;
  user_id?: string;
  event_type: string;
  description: string;
  metadata?: Record<string, unknown>;
  ip_address?: string;
  user_agent?: string;
  created_at: string;
}

interface AuditLogResponse {
  logs: AuditLogEntry[];
  total: number;
  limit: number;
  offset: number;
}

interface SSOConfig {
  id: string;
  org_id: string;
  provider: string;
  enabled: boolean;
  saml_entity_id?: string;
  saml_sso_url?: string;
  saml_slo_url?: string;
  saml_name_id_format?: string;
  // OIDC fields (provider === 'oidc'). The client secret is intentionally absent:
  // the backend never returns it (write-only).
  oidc_issuer_url?: string;
  oidc_client_id?: string;
  // Email domain that pre-login discovery maps to this org.
  email_domain?: string;
  attribute_mapping: Record<string, unknown>;
  require_signed_assertions: boolean;
  require_signed_responses: boolean;
  allow_unsolicited_responses: boolean;
  created_at: string;
  updated_at: string;
}

interface OrgTemplate {
  id: string;
  org_id: string;
  name: string;
  description?: string;
  blueprint_config: Record<string, unknown>;
  security_baseline: Record<string, unknown>;
  created_by: string;
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

interface OrgUsage {
  org_id: string;
  total_requests: number;
  total_storage_gb: number;
  total_ai_tokens: number;
  hosted_mocks_count: number;
  plugins_published: number;
  api_tokens_count: number;
}

interface OrgBilling {
  org_id: string;
  plan: string;
  stripe_customer_id?: string;
  subscription?: {
    id: string;
    status: string;
    current_period_start?: string;
    current_period_end?: string;
    cancel_at_period_end: boolean;
  };
}

interface OrgAiSettings {
  max_ai_calls_per_workspace_per_day: number;
  max_ai_calls_per_workspace_per_month: number;
  feature_flags: {
    ai_studio_enabled: boolean;
    ai_contract_diff_enabled: boolean;
    mockai_enabled: boolean;
    persona_generation_enabled: boolean;
  };
}

// ─── API Helpers ─────────────────────────────────────────────────────────────

const API_BASE = '/api/v1';

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('auth_token');
  return {
    Authorization: `Bearer ${token}`,
    'Content-Type': 'application/json',
  };
}

async function apiFetch<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    ...init,
    headers: { ...authHeaders(), ...init?.headers },
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body.message || body.error || `Request failed (${response.status})`);
  }
  return response.json();
}

// Organizations
const fetchOrganizations = () => apiFetch<Organization[]>(`${API_BASE}/organizations`);
const createOrganization = (data: { name: string; slug: string }) =>
  apiFetch<Organization>(`${API_BASE}/organizations`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
const updateOrganization = (id: string, data: { name?: string; slug?: string }) =>
  apiFetch<Organization>(`${API_BASE}/organizations/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(data),
  });
const deleteOrganization = (id: string) =>
  apiFetch<void>(`${API_BASE}/organizations/${id}`, { method: 'DELETE' });

// Members
const fetchOrgMembers = (orgId: string) =>
  apiFetch<OrgMember[]>(`${API_BASE}/organizations/${orgId}/members`);
const addOrgMember = (orgId: string, data: { email?: string; user_id?: string; role?: string }) =>
  apiFetch<OrgMember>(`${API_BASE}/organizations/${orgId}/members`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
const updateMemberRole = (orgId: string, userId: string, role: string) =>
  apiFetch<OrgMember>(`${API_BASE}/organizations/${orgId}/members/${userId}`, {
    method: 'PATCH',
    body: JSON.stringify({ role }),
  });
const removeMember = (orgId: string, userId: string) =>
  apiFetch<void>(`${API_BASE}/organizations/${orgId}/members/${userId}`, { method: 'DELETE' });

// Invitations
interface PendingInvitation {
  nonce: string;
  email: string;
  role: string;
  created_at: string;
  updated_at: string;
}

const createInvitation = (orgId: string, data: { email: string; role?: string }) =>
  apiFetch<{ token: string; nonce: string; org_id: string; email: string; role: string }>(
    `${API_BASE}/organizations/${orgId}/invitations`,
    { method: 'POST', body: JSON.stringify(data) }
  );
const listInvitations = (orgId: string) =>
  apiFetch<{ invitations: PendingInvitation[] }>(`${API_BASE}/organizations/${orgId}/invitations`);
const revokeInvitation = (orgId: string, nonce: string) =>
  apiFetch<void>(`${API_BASE}/organizations/${orgId}/invitations/${encodeURIComponent(nonce)}`, {
    method: 'DELETE',
  });

// Audit logs
const fetchAuditLogs = (orgId: string, params: { limit?: number; offset?: number; event_type?: string }) => {
  const qs = new URLSearchParams();
  if (params.limit) qs.set('limit', String(params.limit));
  if (params.offset) qs.set('offset', String(params.offset));
  if (params.event_type) qs.set('event_type', params.event_type);
  return apiFetch<AuditLogResponse>(`${API_BASE}/organizations/${orgId}/audit-logs?${qs}`);
};

// SSO
const fetchSSOConfig = () => apiFetch<SSOConfig | null>(`${API_BASE}/sso/config`);
const saveSSOConfig = (data: {
  provider: string;
  saml_entity_id?: string;
  saml_sso_url?: string;
  saml_slo_url?: string;
  saml_x509_cert?: string;
  oidc_issuer_url?: string;
  oidc_client_id?: string;
  oidc_client_secret?: string;
  email_domain?: string;
}) => apiFetch<SSOConfig>(`${API_BASE}/sso/config`, { method: 'POST', body: JSON.stringify(data) });
const deleteSSOConfig = () => apiFetch<void>(`${API_BASE}/sso/config`, { method: 'DELETE' });
const enableSSO = () => apiFetch<void>(`${API_BASE}/sso/enable`, { method: 'POST' });
const disableSSO = () => apiFetch<void>(`${API_BASE}/sso/disable`, { method: 'POST' });

// SSO domain verification (#833): SSO only provisions users whose email domain
// the org has proven it owns via a DNS TXT record. This reports the exact record
// to publish and whether it currently verifies.
interface DomainStatus {
  domain: string;
  record_name: string;
  record_value: string;
  verified: boolean;
}
const fetchDomainStatus = (domain: string) =>
  apiFetch<DomainStatus>(`${API_BASE}/sso/domain/status?domain=${encodeURIComponent(domain)}`);

// Templates
const fetchOrgTemplates = (orgId: string) =>
  apiFetch<{ templates: OrgTemplate[] }>(`${API_BASE}/organizations/${orgId}/templates`);
const createOrgTemplate = (orgId: string, data: { name: string; description?: string; blueprint_config?: Record<string, unknown>; security_baseline?: Record<string, unknown>; is_default?: boolean }) =>
  apiFetch<OrgTemplate>(`${API_BASE}/organizations/${orgId}/templates`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
const updateOrgTemplate = (orgId: string, tid: string, data: { name?: string; description?: string; is_default?: boolean }) =>
  apiFetch<OrgTemplate>(`${API_BASE}/organizations/${orgId}/templates/${tid}`, {
    method: 'PATCH',
    body: JSON.stringify(data),
  });
const deleteOrgTemplate = (orgId: string, tid: string) =>
  apiFetch<void>(`${API_BASE}/organizations/${orgId}/templates/${tid}`, { method: 'DELETE' });

// Quota display helpers — give common keys a friendly label + unit so the
// raw JSON keys from the backend (`requests_per_30d`, `storage_gb`, …)
// don't leak into the UI as snake_case.
const QUOTA_KEY_META: Record<string, { label: string; unit?: 'count' | 'gb' }> = {
  requests_per_30d: { label: 'Requests / 30 days', unit: 'count' },
  requests_per_minute: { label: 'Requests / minute', unit: 'count' },
  storage_gb: { label: 'Storage', unit: 'gb' },
  ai_calls_per_day: { label: 'AI calls / day', unit: 'count' },
  ai_calls_per_month: { label: 'AI calls / month', unit: 'count' },
  hosted_mocks_max: { label: 'Hosted mocks (max)', unit: 'count' },
  max_workspaces: { label: 'Workspaces (max)', unit: 'count' },
  max_members: { label: 'Members (max)', unit: 'count' },
  max_publisher_keys: { label: 'Publisher keys (max)', unit: 'count' },
  api_tokens_max: { label: 'API tokens (max)', unit: 'count' },
};
function formatQuotaKey(key: string): { label: string; unit?: 'count' | 'gb' } {
  const meta = QUOTA_KEY_META[key];
  if (meta) return meta;
  // Fallback: strip underscores, capitalize first letter.
  const label = key.replace(/_/g, ' ');
  return { label: label.charAt(0).toUpperCase() + label.slice(1) };
}
function formatQuotaValue(value: unknown, unit?: 'count' | 'gb'): string {
  if (typeof value === 'number') {
    if (unit === 'gb') return `${value.toLocaleString()} GB`;
    if (unit === 'count') return value.toLocaleString();
    return String(value);
  }
  return String(value);
}

// Usage, Billing, Quota
const fetchOrgUsage = (orgId: string) =>
  apiFetch<OrgUsage>(`${API_BASE}/organizations/${orgId}/usage`);
const fetchOrgBilling = (orgId: string) =>
  apiFetch<OrgBilling>(`${API_BASE}/organizations/${orgId}/billing`);
const fetchOrgQuota = (orgId: string) =>
  apiFetch<{ org_id: string; quota: Record<string, unknown> }>(`${API_BASE}/organizations/${orgId}/quota`);

// AI Settings
const fetchOrgAISettings = (orgId: string) =>
  apiFetch<OrgAiSettings>(`${API_BASE}/organizations/${orgId}/settings/ai`);
const updateOrgAISettings = (orgId: string, data: Partial<OrgAiSettings>) =>
  apiFetch<OrgAiSettings>(`${API_BASE}/organizations/${orgId}/settings/ai`, {
    method: 'PATCH',
    body: JSON.stringify(data),
  });

// Suspicious activity / security events
interface SuspiciousActivity {
  id: string;
  org_id?: string;
  user_id?: string;
  activity_type: string;
  severity: string;
  description: string;
  metadata?: Record<string, unknown>;
  ip_address?: string;
  user_agent?: string;
  resolved: boolean;
  resolved_at?: string;
  created_at: string;
}

interface SuspiciousActivityListResponse {
  activities: SuspiciousActivity[];
  total: number;
}

const fetchSuspiciousActivities = (
  orgId: string,
  params: { severity?: string; limit?: number } = {},
) => {
  const qs = new URLSearchParams();
  if (params.severity) qs.set('severity', params.severity);
  if (params.limit) qs.set('limit', String(params.limit));
  return apiFetch<SuspiciousActivityListResponse>(
    `${API_BASE}/security/suspicious-activities${qs.toString() ? `?${qs}` : ''}`,
    { headers: { 'X-Organization-Id': orgId } },
  );
};

const resolveSuspiciousActivity = (orgId: string, activityId: string) =>
  apiFetch<{ success: boolean; message: string }>(
    `${API_BASE}/security/suspicious-activities/${activityId}/resolve`,
    { method: 'POST', headers: { 'X-Organization-Id': orgId } },
  );

// ─── Helper Components ──────────────────────────────────────────────────────

function getRoleIcon(role: string) {
  switch (role) {
    case 'owner':
      return <Crown className="w-4 h-4 text-warning-500" />;
    case 'admin':
      return <Shield className="w-4 h-4 text-info-500" />;
    default:
      return <User className="w-4 h-4 text-muted-foreground" />;
  }
}

function getRoleBadge(role: string) {
  switch (role) {
    case 'owner':
      return <Badge className="bg-warning-100 text-warning-700 dark:bg-warning-900/30 dark:text-warning-300">Owner</Badge>;
    case 'admin':
      return <Badge className="bg-info-100 text-info-700 dark:bg-info-900/30 dark:text-info-300">Admin</Badge>;
    default:
      return <Badge variant="secondary">Member</Badge>;
  }
}

function getPlanBadge(plan: string) {
  switch (plan) {
    case 'team':
      return <Badge className="bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200">Team</Badge>;
    case 'pro':
      return <Badge className="bg-info-100 text-info-700 dark:bg-info-900/30 dark:text-info-300">Pro</Badge>;
    default:
      return <Badge variant="secondary">Free</Badge>;
  }
}

// ─── Tab Content Components ──────────────────────────────────────────────────

function MembersTab({ org }: { org: Organization }) {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [showAddMember, setShowAddMember] = useState(false);
  const [showInvite, setShowInvite] = useState(false);
  const [addEmail, setAddEmail] = useState('');
  const [addRole, setAddRole] = useState('member');
  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteRole, setInviteRole] = useState('member');
  const [inviteResult, setInviteResult] = useState<{ token: string } | null>(null);
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null);
  const [editingRole, setEditingRole] = useState<string | null>(null);

  const { data: members, isLoading } = useQuery({
    queryKey: ['org-members', org.id],
    queryFn: () => fetchOrgMembers(org.id),
  });

  const addMemberMutation = useMutation({
    mutationFn: (data: { email: string; role: string }) => addOrgMember(org.id, data),
    onSuccess: () => {
      showToast('success', 'Member added', `Successfully added ${addEmail}`);
      setAddEmail('');
      setAddRole('member');
      setShowAddMember(false);
      queryClient.invalidateQueries({ queryKey: ['org-members', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to add member', err.message),
  });

  const updateRoleMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: string }) =>
      updateMemberRole(org.id, userId, role),
    onSuccess: () => {
      showToast('success', 'Role updated');
      setEditingRole(null);
      queryClient.invalidateQueries({ queryKey: ['org-members', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to update role', err.message),
  });

  const removeMemberMutation = useMutation({
    mutationFn: (userId: string) => removeMember(org.id, userId),
    onSuccess: () => {
      showToast('success', 'Member removed');
      setConfirmRemove(null);
      queryClient.invalidateQueries({ queryKey: ['org-members', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to remove member', err.message),
  });

  const inviteMutation = useMutation({
    mutationFn: (data: { email: string; role: string }) => createInvitation(org.id, data),
    onSuccess: (data) => {
      showToast('success', 'Invitation created');
      setInviteResult(data);
    },
    onError: (err: Error) => showToast('error', 'Failed to create invitation', err.message),
  });

  return (
    <div className="space-y-4">
      <div className="flex gap-2 justify-end">
        <Button size="sm" variant="outline" onClick={() => setShowInvite(true)}>
          <Link className="w-4 h-4 mr-2" />
          Invite Link
        </Button>
        <Button size="sm" onClick={() => setShowAddMember(true)}>
          <UserPlus className="w-4 h-4 mr-2" />
          Add Member
        </Button>
      </div>

      {isLoading ? (
        <div className="text-center py-4 text-muted-foreground">Loading members...</div>
      ) : members && members.length > 0 ? (
        <div className="space-y-2">
          {members.map((member) => (
            <div key={member.id} className="flex items-center justify-between p-3 border rounded-lg">
              <div className="flex items-center space-x-3">
                {getRoleIcon(member.role)}
                <div>
                  <div className="font-medium">{member.username}</div>
                  <div className="text-sm text-muted-foreground">{member.email}</div>
                </div>
              </div>
              <div className="flex items-center gap-2">
                {editingRole === member.user_id && member.role !== 'owner' ? (
                  <div className="flex items-center gap-1">
                    <select
                      className="text-sm border rounded px-2 py-1 bg-background"
                      defaultValue={member.role}
                      onChange={(e) =>
                        updateRoleMutation.mutate({ userId: member.user_id, role: e.target.value })
                      }
                    >
                      <option value="admin">Admin</option>
                      <option value="member">Member</option>
                    </select>
                    <Button size="sm" variant="ghost" onClick={() => setEditingRole(null)}>
                      <X className="w-3 h-3" />
                    </Button>
                  </div>
                ) : (
                  <button
                    className="cursor-pointer"
                    onClick={() => member.role !== 'owner' && setEditingRole(member.user_id)}
                    title={member.role === 'owner' ? 'Owner role cannot be changed' : 'Click to change role'}
                  >
                    {getRoleBadge(member.role)}
                  </button>
                )}
                {member.role !== 'owner' && (
                  confirmRemove === member.user_id ? (
                    <div className="flex items-center gap-1">
                      <Button
                        size="sm"
                        variant="destructive"
                        onClick={() => removeMemberMutation.mutate(member.user_id)}
                        disabled={removeMemberMutation.isPending}
                      >
                        {removeMemberMutation.isPending ? 'Removing...' : 'Confirm'}
                      </Button>
                      <Button size="sm" variant="outline" onClick={() => setConfirmRemove(null)}>
                        Cancel
                      </Button>
                    </div>
                  ) : (
                    <Button
                      size="sm"
                      variant="ghost"
                      className="text-destructive hover:text-destructive"
                      onClick={() => setConfirmRemove(member.user_id)}
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  )
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-center py-8 text-muted-foreground">No members found</div>
      )}

      {/* Add Member Dialog */}
      <Dialog open={showAddMember} onOpenChange={setShowAddMember}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Add Member</DialogTitle>
            <DialogDescription>Add an existing user to this organization by email</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>Email</Label>
              <Input
                type="email"
                placeholder="user@example.com"
                value={addEmail}
                onChange={(e) => setAddEmail(e.target.value)}
              />
            </div>
            <div>
              <Label>Role</Label>
              <select
                className="w-full border rounded px-3 py-2 bg-background mt-1"
                value={addRole}
                onChange={(e) => setAddRole(e.target.value)}
              >
                <option value="member">Member</option>
                <option value="admin">Admin</option>
              </select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddMember(false)}>Cancel</Button>
            <Button
              onClick={() => addMemberMutation.mutate({ email: addEmail, role: addRole })}
              disabled={!addEmail.trim() || addMemberMutation.isPending}
            >
              {addMemberMutation.isPending ? 'Adding...' : 'Add Member'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Invite Link Dialog */}
      <Dialog open={showInvite} onOpenChange={(open) => { setShowInvite(open); if (!open) setInviteResult(null); }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Create Invitation Link</DialogTitle>
            <DialogDescription>Generate a shareable invitation link for new members</DialogDescription>
          </DialogHeader>
          {inviteResult ? (
            <div className="space-y-4">
              <div>
                <Label>Invitation Link</Label>
                <div className="flex gap-2 mt-1">
                  <Input readOnly value={`${window.location.origin}/invite/${inviteResult.token}`} />
                  <Button
                    variant="outline"
                    onClick={() => {
                      navigator.clipboard.writeText(`${window.location.origin}/invite/${inviteResult.token}`);
                      showToast('success', 'Copied to clipboard');
                    }}
                  >
                    <Copy className="w-4 h-4" />
                  </Button>
                </div>
              </div>
              <p className="text-sm text-muted-foreground">Share this link with the person you want to invite.</p>
            </div>
          ) : (
            <div className="space-y-4">
              <div>
                <Label>Email</Label>
                <Input
                  type="email"
                  placeholder="user@example.com"
                  value={inviteEmail}
                  onChange={(e) => setInviteEmail(e.target.value)}
                />
              </div>
              <div>
                <Label>Role</Label>
                <select
                  className="w-full border rounded px-3 py-2 bg-background mt-1"
                  value={inviteRole}
                  onChange={(e) => setInviteRole(e.target.value)}
                >
                  <option value="member">Member</option>
                  <option value="admin">Admin</option>
                </select>
              </div>
            </div>
          )}
          <DialogFooter>
            {inviteResult ? (
              <Button onClick={() => { setShowInvite(false); setInviteResult(null); setInviteEmail(''); }}>Done</Button>
            ) : (
              <>
                <Button variant="outline" onClick={() => setShowInvite(false)}>Cancel</Button>
                <Button
                  onClick={() => inviteMutation.mutate({ email: inviteEmail, role: inviteRole })}
                  disabled={!inviteEmail.trim() || inviteMutation.isPending}
                >
                  {inviteMutation.isPending ? 'Creating...' : 'Create Invitation'}
                </Button>
              </>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function SettingsTab({ org }: { org: Organization }) {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(org.name);
  const [slug, setSlug] = useState(org.slug);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  useEffect(() => {
    setName(org.name);
    setSlug(org.slug);
    setEditing(false);
  }, [org.id, org.name, org.slug]);

  const updateMutation = useMutation({
    mutationFn: () => updateOrganization(org.id, { name, slug }),
    onSuccess: () => {
      showToast('success', 'Organization updated');
      setEditing(false);
      queryClient.invalidateQueries({ queryKey: ['organizations'] });
    },
    onError: (err: Error) => showToast('error', 'Failed to update', err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: () => deleteOrganization(org.id),
    onSuccess: () => {
      showToast('success', 'Organization deleted');
      queryClient.invalidateQueries({ queryKey: ['organizations'] });
    },
    onError: (err: Error) => showToast('error', 'Failed to delete', err.message),
  });

  return (
    <div className="space-y-6">
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h4 className="text-sm font-semibold">Organization Details</h4>
          {!editing && (
            <Button size="sm" variant="ghost" onClick={() => setEditing(true)}>
              <Edit className="w-4 h-4 mr-1" />
              Edit
            </Button>
          )}
        </div>
        <div>
          <Label>Organization Name</Label>
          <Input value={name} disabled={!editing} onChange={(e) => setName(e.target.value)} />
        </div>
        <div>
          <Label>Slug</Label>
          <Input value={slug} disabled={!editing} onChange={(e) => setSlug(e.target.value)} />
        </div>
        <div>
          <Label>Plan</Label>
          <div className="mt-2">{getPlanBadge(org.plan)}</div>
        </div>
        <div>
          <Label>Created</Label>
          <div className="text-sm text-muted-foreground mt-1">
            {new Date(org.created_at).toLocaleDateString()}
          </div>
        </div>
        {editing && (
          <div className="flex gap-2">
            <Button onClick={() => updateMutation.mutate()} disabled={updateMutation.isPending}>
              <Save className="w-4 h-4 mr-2" />
              {updateMutation.isPending ? 'Saving...' : 'Save'}
            </Button>
            <Button variant="outline" onClick={() => { setEditing(false); setName(org.name); setSlug(org.slug); }}>
              Cancel
            </Button>
          </div>
        )}
      </div>

      <div className="border-t pt-6">
        <h4 className="text-sm font-semibold text-destructive mb-2">Danger Zone</h4>
        {showDeleteConfirm ? (
          <div className="flex items-center gap-3 p-3 border border-destructive rounded-lg">
            <p className="text-sm text-muted-foreground flex-1">
              This will permanently delete <strong>{org.name}</strong> and all associated data. This cannot be undone.
            </p>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => deleteMutation.mutate()}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? 'Deleting...' : 'Confirm Delete'}
            </Button>
            <Button variant="outline" size="sm" onClick={() => setShowDeleteConfirm(false)}>
              Cancel
            </Button>
          </div>
        ) : (
          <Button
            variant="ghost"
            className="text-destructive hover:text-destructive"
            onClick={() => setShowDeleteConfirm(true)}
          >
            <Trash2 className="w-4 h-4 mr-2" />
            Delete Organization
          </Button>
        )}
      </div>
    </div>
  );
}

// Format a snake_case audit event type into a human-readable label.
// Special-cases acronyms ("api", "sso", "byok") so we get "API Token Created"
// rather than "Api Token Created".
const ACRONYMS = new Set(['api', 'sso', 'byok', 'ip']);
function humanizeEventType(eventType: string): string {
  return eventType
    .split('_')
    .map((part) =>
      ACRONYMS.has(part) ? part.toUpperCase() : part.charAt(0).toUpperCase() + part.slice(1),
    )
    .join(' ');
}

// Filter dropdown options. Values must match the backend's snake_case
// `AuditEventType::from_str` mapping or the filter is silently a no-op.
// The backend accepts a comma-separated list, so a "group" option (e.g. all
// API token events) is just a CSV value.
const AUDIT_EVENT_TYPE_OPTIONS: Array<{ value: string; label: string }> = [
  { value: 'api_token_created,api_token_deleted,api_token_rotated', label: 'API Token (any)' },
  { value: 'api_token_created', label: 'API Token Created' },
  { value: 'api_token_deleted', label: 'API Token Deleted' },
  { value: 'api_token_rotated', label: 'API Token Rotated' },
  { value: 'org_updated', label: 'Organization Updated' },
  { value: 'org_deleted', label: 'Organization Deleted' },
  { value: 'member_added', label: 'Member Added' },
  { value: 'member_removed', label: 'Member Removed' },
  { value: 'member_role_changed', label: 'Role Changed' },
  { value: 'settings_updated', label: 'Settings Updated' },
  { value: 'org_plan_changed', label: 'Plan Changed' },
  {
    value: 'billing_checkout,billing_upgrade,billing_downgrade,billing_canceled',
    label: 'Billing (any)',
  },
  { value: 'billing_checkout', label: 'Billing Checkout Started' },
  { value: 'billing_upgrade', label: 'Billing Upgrade' },
  { value: 'billing_downgrade', label: 'Billing Downgrade' },
  { value: 'billing_canceled', label: 'Billing Canceled' },
  { value: 'byok_config_updated', label: 'BYOK Config Updated' },
  { value: 'byok_config_deleted', label: 'BYOK Config Deleted' },
  { value: 'deployment_created', label: 'Deployment Created' },
  { value: 'deployment_deleted', label: 'Deployment Deleted' },
  { value: 'password_changed', label: 'Password Changed' },
  { value: 'two_factor_enabled', label: 'Two-Factor Enabled' },
  { value: 'two_factor_disabled', label: 'Two-Factor Disabled' },
];

function AuditLogTab({ org }: { org: Organization }) {
  const [searchParams] = useSearchParams();
  const [offset, setOffset] = useState(0);
  const [eventTypeFilter, setEventTypeFilter] = useState(
    () => searchParams.get('event_type') ?? '',
  );
  const limit = 20;

  const { data, isLoading } = useQuery({
    queryKey: ['audit-logs', org.id, offset, eventTypeFilter],
    queryFn: () => fetchAuditLogs(org.id, { limit, offset, event_type: eventTypeFilter || undefined }),
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Label className="text-sm">Filter by event:</Label>
        <select
          className="border rounded px-2 py-1 text-sm bg-background"
          value={eventTypeFilter}
          onChange={(e) => { setEventTypeFilter(e.target.value); setOffset(0); }}
        >
          <option value="">All events</option>
          {AUDIT_EVENT_TYPE_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>{opt.label}</option>
          ))}
        </select>
      </div>

      {isLoading ? (
        <div className="text-center py-4 text-muted-foreground">Loading audit logs...</div>
      ) : data && data.logs.length > 0 ? (
        <>
          <div className="space-y-2">
            {data.logs.map((log) => (
              <div key={log.id} className="p-3 border rounded-lg text-sm">
                <div className="flex items-center justify-between mb-1">
                  <Badge variant="secondary" className="text-xs" title={log.event_type}>
                    {humanizeEventType(log.event_type)}
                  </Badge>
                  <span className="text-xs text-muted-foreground">
                    {new Date(log.created_at).toLocaleString()}
                  </span>
                </div>
                <p className="text-muted-foreground">{log.description}</p>
                {log.ip_address && (
                  <p className="text-xs text-muted-foreground mt-1">IP: {log.ip_address}</p>
                )}
              </div>
            ))}
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              Showing {offset + 1}-{Math.min(offset + limit, data.total)} of {data.total}
            </span>
            <div className="flex gap-2">
              <Button
                size="sm"
                variant="outline"
                disabled={offset === 0}
                onClick={() => setOffset(Math.max(0, offset - limit))}
              >
                <ChevronLeft className="w-4 h-4" />
              </Button>
              <Button
                size="sm"
                variant="outline"
                disabled={offset + limit >= data.total}
                onClick={() => setOffset(offset + limit)}
              >
                <ChevronRight className="w-4 h-4" />
              </Button>
            </div>
          </div>
        </>
      ) : (
        <div className="text-center py-8 text-muted-foreground">No audit logs found</div>
      )}
    </div>
  );
}

function SecurityActivityTab({ org }: { org: Organization }) {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [severityFilter, setSeverityFilter] = useState('');

  const { data, isLoading } = useQuery({
    queryKey: ['suspicious-activities', org.id, severityFilter],
    queryFn: () =>
      fetchSuspiciousActivities(org.id, {
        severity: severityFilter || undefined,
        limit: 100,
      }),
  });

  const resolveMutation = useMutation({
    mutationFn: (activityId: string) => resolveSuspiciousActivity(org.id, activityId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['suspicious-activities', org.id] });
      showToast('Marked as resolved', 'success');
    },
    onError: (err: Error) => showToast(err.message, 'error'),
  });

  const severityBadge = (severity: string) => {
    const s = severity.toLowerCase();
    if (s === 'critical' || s === 'high') {
      return (
        <Badge className="bg-danger-100 text-danger-700 dark:bg-danger-900/30 dark:text-danger-300">
          {severity}
        </Badge>
      );
    }
    if (s === 'medium') {
      return (
        <Badge className="bg-warning-100 text-warning-700 dark:bg-warning-900/30 dark:text-warning-300">
          {severity}
        </Badge>
      );
    }
    return <Badge variant="secondary">{severity}</Badge>;
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Label className="text-sm">Severity:</Label>
        <select
          className="border rounded px-2 py-1 text-sm bg-background"
          value={severityFilter}
          onChange={(e) => setSeverityFilter(e.target.value)}
        >
          <option value="">All severities</option>
          <option value="critical">Critical</option>
          <option value="high">High</option>
          <option value="medium">Medium</option>
          <option value="low">Low</option>
        </select>
        <span className="ml-auto text-xs text-muted-foreground">
          {data ? `${data.total} unresolved` : ''}
        </span>
      </div>

      {isLoading ? (
        <div className="text-center py-4 text-muted-foreground">Loading security events…</div>
      ) : data && data.activities.length > 0 ? (
        <div className="space-y-2">
          {data.activities.map((activity) => (
            <div key={activity.id} className="p-3 border rounded-lg text-sm">
              <div className="flex items-center justify-between mb-1">
                <div className="flex items-center gap-2">
                  <AlertTriangle className="w-4 h-4 text-warning-600" />
                  <Badge variant="secondary" className="text-xs">
                    {activity.activity_type}
                  </Badge>
                  {severityBadge(activity.severity)}
                </div>
                <span className="text-xs text-muted-foreground">
                  {new Date(activity.created_at).toLocaleString()}
                </span>
              </div>
              <p className="text-muted-foreground">{activity.description}</p>
              <div className="mt-2 flex flex-wrap gap-3 text-xs text-muted-foreground">
                {activity.ip_address && <span>IP: {activity.ip_address}</span>}
                {activity.user_agent && (
                  <span className="truncate max-w-md" title={activity.user_agent}>
                    UA: {activity.user_agent}
                  </span>
                )}
              </div>
              <div className="mt-2 flex justify-end">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={resolveMutation.isPending}
                  onClick={() => resolveMutation.mutate(activity.id)}
                >
                  <CheckCircle2 className="w-4 h-4 mr-1" />
                  Mark resolved
                </Button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-center py-8 text-muted-foreground">
          No unresolved suspicious activities.
        </div>
      )}
    </div>
  );
}

function TemplatesTab({ org }: { org: Organization }) {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [showCreate, setShowCreate] = useState(false);
  const [showEdit, setShowEdit] = useState<OrgTemplate | null>(null);
  const [templateName, setTemplateName] = useState('');
  const [templateDesc, setTemplateDesc] = useState('');
  const [templateDefault, setTemplateDefault] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  const { data, isLoading } = useQuery({
    queryKey: ['org-templates', org.id],
    queryFn: () => fetchOrgTemplates(org.id),
  });

  const createMutation = useMutation({
    mutationFn: () => createOrgTemplate(org.id, { name: templateName, description: templateDesc || undefined, is_default: templateDefault }),
    onSuccess: () => {
      showToast('success', 'Template created');
      setShowCreate(false);
      setTemplateName('');
      setTemplateDesc('');
      setTemplateDefault(false);
      queryClient.invalidateQueries({ queryKey: ['org-templates', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to create template', err.message),
  });

  const updateMutation = useMutation({
    mutationFn: () => {
      if (!showEdit) throw new Error('No template selected');
      return updateOrgTemplate(org.id, showEdit.id, { name: templateName, description: templateDesc || undefined, is_default: templateDefault });
    },
    onSuccess: () => {
      showToast('success', 'Template updated');
      setShowEdit(null);
      queryClient.invalidateQueries({ queryKey: ['org-templates', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to update template', err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (tid: string) => deleteOrgTemplate(org.id, tid),
    onSuccess: () => {
      showToast('success', 'Template deleted');
      setConfirmDelete(null);
      queryClient.invalidateQueries({ queryKey: ['org-templates', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to delete template', err.message),
  });

  const openEditDialog = (t: OrgTemplate) => {
    setTemplateName(t.name);
    setTemplateDesc(t.description || '');
    setTemplateDefault(t.is_default);
    setShowEdit(t);
  };

  const closeDialogs = () => {
    setShowCreate(false);
    setShowEdit(null);
    setTemplateName('');
    setTemplateDesc('');
    setTemplateDefault(false);
  };

  const templateForm = (
    <div className="space-y-4">
      <div>
        <Label>Name</Label>
        <Input value={templateName} onChange={(e) => setTemplateName(e.target.value)} placeholder="Template name" />
      </div>
      <div>
        <Label>Description</Label>
        <Input value={templateDesc} onChange={(e) => setTemplateDesc(e.target.value)} placeholder="Optional description" />
      </div>
      <div className="flex items-center gap-2">
        <input
          type="checkbox"
          id="template-default"
          checked={templateDefault}
          onChange={(e) => setTemplateDefault(e.target.checked)}
          className="rounded"
        />
        <Label htmlFor="template-default">Set as default template</Label>
      </div>
    </div>
  );

  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button size="sm" onClick={() => setShowCreate(true)}>
          <Plus className="w-4 h-4 mr-2" />
          New Template
        </Button>
      </div>

      {isLoading ? (
        <div className="text-center py-4 text-muted-foreground">Loading templates...</div>
      ) : data && data.templates.length > 0 ? (
        <div className="space-y-2">
          {data.templates.map((t) => (
            <div key={t.id} className="flex items-center justify-between p-3 border rounded-lg">
              <div>
                <div className="font-medium flex items-center gap-2">
                  <FileText className="w-4 h-4 text-muted-foreground" />
                  {t.name}
                  {t.is_default && <Badge variant="secondary" className="text-xs">Default</Badge>}
                </div>
                {t.description && <p className="text-sm text-muted-foreground mt-1">{t.description}</p>}
              </div>
              <div className="flex items-center gap-1">
                <Button size="sm" variant="ghost" onClick={() => openEditDialog(t)}>
                  <Edit className="w-4 h-4" />
                </Button>
                {confirmDelete === t.id ? (
                  <div className="flex items-center gap-1">
                    <Button size="sm" variant="destructive" onClick={() => deleteMutation.mutate(t.id)} disabled={deleteMutation.isPending}>
                      {deleteMutation.isPending ? '...' : 'Yes'}
                    </Button>
                    <Button size="sm" variant="outline" onClick={() => setConfirmDelete(null)}>No</Button>
                  </div>
                ) : (
                  <Button size="sm" variant="ghost" className="text-destructive hover:text-destructive" onClick={() => setConfirmDelete(t.id)}>
                    <Trash2 className="w-4 h-4" />
                  </Button>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-center py-8 text-muted-foreground">No templates yet</div>
      )}

      {/* Create Template Dialog */}
      <Dialog open={showCreate} onOpenChange={(open) => { if (!open) closeDialogs(); else setShowCreate(true); }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Create Template</DialogTitle>
            <DialogDescription>Create a new organization template</DialogDescription>
          </DialogHeader>
          {templateForm}
          <DialogFooter>
            <Button variant="outline" onClick={closeDialogs}>Cancel</Button>
            <Button onClick={() => createMutation.mutate()} disabled={!templateName.trim() || createMutation.isPending}>
              {createMutation.isPending ? 'Creating...' : 'Create'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Template Dialog */}
      <Dialog open={!!showEdit} onOpenChange={(open) => { if (!open) closeDialogs(); }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Edit Template</DialogTitle>
            <DialogDescription>Update template details</DialogDescription>
          </DialogHeader>
          {templateForm}
          <DialogFooter>
            <Button variant="outline" onClick={closeDialogs}>Cancel</Button>
            <Button onClick={() => updateMutation.mutate()} disabled={!templateName.trim() || updateMutation.isPending}>
              {updateMutation.isPending ? 'Saving...' : 'Save'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function DomainVerificationCard() {
  const { showToast } = useToast();
  const [domain, setDomain] = useState('');
  const [status, setStatus] = useState<DomainStatus | null>(null);

  const checkMutation = useMutation({
    mutationFn: () => fetchDomainStatus(domain.trim()),
    onSuccess: (data) => {
      setStatus(data);
      showToast(
        data.verified ? 'success' : 'info',
        data.verified ? `${data.domain} is verified` : `${data.domain} is not verified yet`,
      );
    },
    onError: (err: Error) => showToast('error', 'Domain check failed', err.message),
  });

  const copy = (text: string) => {
    navigator.clipboard?.writeText(text);
    showToast('success', 'Copied to clipboard');
  };

  return (
    <div className="space-y-3 border-t pt-4">
      <h4 className="text-sm font-semibold flex items-center gap-2">
        <Shield className="w-4 h-4" /> Domain Verification
      </h4>
      <p className="text-sm text-muted-foreground">
        SSO only creates or links accounts for email domains your organization has verified.
        Enter a domain, publish the DNS TXT record shown, then check.
      </p>
      <div className="flex gap-2">
        <Input
          value={domain}
          onChange={(e) => {
            setDomain(e.target.value);
            setStatus(null);
          }}
          placeholder="acme.com"
        />
        <Button onClick={() => checkMutation.mutate()} disabled={!domain.trim() || checkMutation.isPending}>
          {checkMutation.isPending ? 'Checking...' : 'Check'}
        </Button>
      </div>
      {status && (
        <div className="p-3 border rounded-lg bg-muted/50 text-sm space-y-2">
          <div className="flex items-center gap-2">
            {status.verified ? (
              <Badge className="bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-300">
                <CheckCircle2 className="w-3.5 h-3.5 mr-1" /> Verified
              </Badge>
            ) : (
              <Badge className="bg-warning-100 text-warning-700 dark:bg-warning-900/30 dark:text-warning-300">
                <AlertTriangle className="w-3.5 h-3.5 mr-1" /> Pending
              </Badge>
            )}
            <span className="text-muted-foreground">{status.domain}</span>
          </div>
          <div className="space-y-1">
            <div className="text-muted-foreground text-xs">Add this DNS TXT record:</div>
            <div className="flex items-center gap-2">
              <span className="text-muted-foreground text-xs w-12">Name</span>
              <code className="text-xs flex-1 break-all">{status.record_name}</code>
              <Button size="sm" variant="ghost" onClick={() => copy(status.record_name)}>
                <Copy className="w-3.5 h-3.5" />
              </Button>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-muted-foreground text-xs w-12">Value</span>
              <code className="text-xs flex-1 break-all">{status.record_value}</code>
              <Button size="sm" variant="ghost" onClick={() => copy(status.record_value)}>
                <Copy className="w-3.5 h-3.5" />
              </Button>
            </div>
          </div>
          {!status.verified && (
            <p className="text-xs text-muted-foreground">
              DNS changes can take a few minutes to propagate. Check again after publishing.
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function SSOTab({ org }: { org: Organization }) {
  const { showToast } = useToast();
  const queryClient = useQueryClient();

  // Provider selector: 'saml' | 'oidc'
  const [provider, setProvider] = useState<'saml' | 'oidc'>('saml');

  // SAML fields
  const [entityId, setEntityId] = useState('');
  const [ssoUrl, setSsoUrl] = useState('');
  const [sloUrl, setSloUrl] = useState('');
  const [x509Cert, setX509Cert] = useState('');

  // OIDC fields
  const [oidcIssuerUrl, setOidcIssuerUrl] = useState('');
  const [oidcClientId, setOidcClientId] = useState('');
  // oidcClientSecret is write-only: never pre-populated from the server response
  const [oidcClientSecret, setOidcClientSecret] = useState('');

  // Email domain (shown for both providers; routes pre-login discovery to this org)
  const [emailDomain, setEmailDomain] = useState('');

  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const { data: ssoConfig, isLoading } = useQuery({
    queryKey: ['sso-config', org.id],
    queryFn: fetchSSOConfig,
    enabled: org.plan === 'team',
  });

  useEffect(() => {
    if (ssoConfig) {
      setProvider(ssoConfig.provider === 'oidc' ? 'oidc' : 'saml');
      setEntityId(ssoConfig.saml_entity_id || '');
      setSsoUrl(ssoConfig.saml_sso_url || '');
      setSloUrl(ssoConfig.saml_slo_url || '');
      setOidcIssuerUrl(ssoConfig.oidc_issuer_url || '');
      setOidcClientId(ssoConfig.oidc_client_id || '');
      setEmailDomain(ssoConfig.email_domain || '');
    }
  }, [ssoConfig]);

  const saveMutation = useMutation({
    mutationFn: () => {
      const base = { provider, email_domain: emailDomain || undefined };
      if (provider === 'saml') {
        return saveSSOConfig({
          ...base,
          saml_entity_id: entityId,
          saml_sso_url: ssoUrl,
          saml_slo_url: sloUrl || undefined,
          saml_x509_cert: x509Cert || undefined,
        });
      }
      return saveSSOConfig({
        ...base,
        oidc_issuer_url: oidcIssuerUrl,
        oidc_client_id: oidcClientId,
        oidc_client_secret: oidcClientSecret || undefined,
      });
    },
    onSuccess: () => {
      showToast('success', 'SSO configuration saved');
      setX509Cert('');
      setOidcClientSecret('');
      queryClient.invalidateQueries({ queryKey: ['sso-config', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to save SSO config', err.message),
  });

  const toggleMutation = useMutation({
    mutationFn: () => (ssoConfig?.enabled ? disableSSO() : enableSSO()),
    onSuccess: () => {
      showToast('success', `SSO ${ssoConfig?.enabled ? 'disabled' : 'enabled'}`);
      queryClient.invalidateQueries({ queryKey: ['sso-config', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to toggle SSO', err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: deleteSSOConfig,
    onSuccess: () => {
      showToast('success', 'SSO configuration deleted');
      setShowDeleteConfirm(false);
      setEntityId('');
      setSsoUrl('');
      setSloUrl('');
      setOidcIssuerUrl('');
      setOidcClientId('');
      setEmailDomain('');
      queryClient.invalidateQueries({ queryKey: ['sso-config', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to delete SSO config', err.message),
  });

  if (org.plan !== 'team') {
    return (
      <div className="text-center py-8">
        <Lock className="w-8 h-8 mx-auto text-muted-foreground mb-3" />
        <h4 className="font-semibold mb-1">SSO requires Team plan</h4>
        <p className="text-sm text-muted-foreground">
          Upgrade to the Team plan to configure Single Sign-On for your organization.
        </p>
      </div>
    );
  }

  if (isLoading) {
    return <div className="text-center py-4 text-muted-foreground">Loading SSO configuration...</div>;
  }

  // Save button enabled: require the provider-specific required fields.
  const isSaveEnabled =
    provider === 'saml'
      ? Boolean(entityId.trim() && ssoUrl.trim())
      : Boolean(oidcIssuerUrl.trim() && oidcClientId.trim());

  return (
    <div className="space-y-6">
      {ssoConfig && (
        <div className="flex items-center justify-between p-3 border rounded-lg">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium">SSO Status</span>
            <Badge className={ssoConfig.enabled ? 'bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-300' : ''}>
              {ssoConfig.enabled ? 'Enabled' : 'Disabled'}
            </Badge>
          </div>
          <Button
            size="sm"
            variant="outline"
            onClick={() => toggleMutation.mutate()}
            disabled={toggleMutation.isPending}
          >
            {ssoConfig.enabled ? <ToggleRight className="w-4 h-4 mr-2" /> : <ToggleLeft className="w-4 h-4 mr-2" />}
            {ssoConfig.enabled ? 'Disable' : 'Enable'}
          </Button>
        </div>
      )}

      {/* ── Provider selector ── */}
      <div className="space-y-2">
        <Label>SSO Provider</Label>
        <div className="flex gap-2">
          {(['saml', 'oidc'] as const).map((p) => (
            <button
              key={p}
              type="button"
              onClick={() => setProvider(p)}
              className={`px-4 py-2 text-sm rounded border transition-colors ${
                provider === p
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'bg-background text-foreground border-border hover:bg-muted'
              }`}
            >
              {p === 'saml' ? 'SAML 2.0' : 'OpenID Connect (OIDC)'}
            </button>
          ))}
        </div>
      </div>

      {/* ── SAML fields ── */}
      {provider === 'saml' && (
        <div className="space-y-4">
          <h4 className="text-sm font-semibold">SAML 2.0 Configuration</h4>
          <div>
            <Label>Entity ID (Issuer)</Label>
            <Input value={entityId} onChange={(e) => setEntityId(e.target.value)} placeholder="https://idp.example.com/metadata" />
          </div>
          <div>
            <Label>SSO URL (Login URL)</Label>
            <Input value={ssoUrl} onChange={(e) => setSsoUrl(e.target.value)} placeholder="https://idp.example.com/sso" />
          </div>
          <div>
            <Label>SLO URL (Logout URL, optional)</Label>
            <Input value={sloUrl} onChange={(e) => setSloUrl(e.target.value)} placeholder="https://idp.example.com/slo" />
          </div>
          <div>
            <Label>X.509 Certificate (paste new cert to update)</Label>
            <textarea
              className="w-full border rounded px-3 py-2 text-sm bg-background font-mono min-h-[100px] mt-1"
              value={x509Cert}
              onChange={(e) => setX509Cert(e.target.value)}
              placeholder="-----BEGIN CERTIFICATE-----&#10;...&#10;-----END CERTIFICATE-----"
            />
          </div>
        </div>
      )}

      {/* ── OIDC fields ── */}
      {provider === 'oidc' && (
        <div className="space-y-4">
          <h4 className="text-sm font-semibold">OpenID Connect Configuration</h4>
          <div>
            <Label>Issuer URL</Label>
            <Input
              value={oidcIssuerUrl}
              onChange={(e) => setOidcIssuerUrl(e.target.value)}
              placeholder="https://accounts.google.com"
            />
            <p className="text-xs text-muted-foreground mt-1">The OIDC provider&apos;s issuer URL (must expose a /.well-known/openid-configuration endpoint).</p>
          </div>
          <div>
            <Label>Client ID</Label>
            <Input
              value={oidcClientId}
              onChange={(e) => setOidcClientId(e.target.value)}
              placeholder="your-client-id"
            />
          </div>
          <div>
            <Label>Client Secret</Label>
            <Input
              type="password"
              value={oidcClientSecret}
              onChange={(e) => setOidcClientSecret(e.target.value)}
              placeholder="Leave blank to keep existing secret"
              autoComplete="new-password"
            />
            <p className="text-xs text-muted-foreground mt-1">Write-only — the secret is never returned by the API. Leave blank to keep the stored secret unchanged.</p>
          </div>
        </div>
      )}

      {/* ── Email domain (always visible; routes pre-login discovery to this org) ── */}
      <div className="space-y-2">
        <h4 className="text-sm font-semibold">Email Domain</h4>
        <div>
          <Label>Domain</Label>
          <Input
            value={emailDomain}
            onChange={(e) => setEmailDomain(e.target.value)}
            placeholder="company.com"
          />
          <p className="text-xs text-muted-foreground mt-1">
            Users signing in with an email from this domain will be redirected to SSO automatically.
            Ownership of this domain must be verified below before SSO provisions accounts.
          </p>
        </div>
      </div>

      {/* ── Save button ── */}
      <div className="flex gap-2">
        <Button onClick={() => saveMutation.mutate()} disabled={!isSaveEnabled || saveMutation.isPending}>
          <Save className="w-4 h-4 mr-2" />
          {saveMutation.isPending ? 'Saving...' : 'Save Configuration'}
        </Button>
      </div>

      {/* ── Service Provider details (SAML only) ── */}
      {ssoConfig && provider === 'saml' && (
        <div className="space-y-3">
          <h4 className="text-sm font-semibold">Service Provider Details</h4>
          <div className="p-3 border rounded-lg bg-muted/50 text-sm space-y-2">
            <div>
              <span className="text-muted-foreground">SAML Metadata URL: </span>
              <code className="text-xs">{window.location.origin}/api/v1/sso/saml/metadata/{org.slug}</code>
            </div>
            <div>
              <span className="text-muted-foreground">ACS URL: </span>
              <code className="text-xs">{window.location.origin}/api/v1/sso/saml/acs/{org.slug}</code>
            </div>
            <div>
              <span className="text-muted-foreground">SLO URL: </span>
              <code className="text-xs">{window.location.origin}/api/v1/sso/saml/slo/{org.slug}</code>
            </div>
          </div>
        </div>
      )}

      <DomainVerificationCard />

      {ssoConfig && (
        <div className="border-t pt-4">
          {showDeleteConfirm ? (
            <div className="flex items-center gap-3">
              <p className="text-sm text-muted-foreground flex-1">Remove SSO configuration? Users will need to log in with email/password.</p>
              <Button variant="destructive" size="sm" onClick={() => deleteMutation.mutate()} disabled={deleteMutation.isPending}>
                {deleteMutation.isPending ? 'Deleting...' : 'Confirm Delete'}
              </Button>
              <Button variant="outline" size="sm" onClick={() => setShowDeleteConfirm(false)}>Cancel</Button>
            </div>
          ) : (
            <Button variant="ghost" className="text-destructive hover:text-destructive" onClick={() => setShowDeleteConfirm(true)}>
              <Trash2 className="w-4 h-4 mr-2" />
              Remove SSO Configuration
            </Button>
          )}
        </div>
      )}
    </div>
  );
}

function InvitationsTab({ org }: { org: Organization }) {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [showCreate, setShowCreate] = useState(false);
  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteRole, setInviteRole] = useState('member');
  const [inviteResult, setInviteResult] = useState<{ token: string } | null>(null);

  const { data: invitationData, isLoading } = useQuery({
    queryKey: ['org-invitations', org.id],
    queryFn: () => listInvitations(org.id),
  });
  const invitations = invitationData?.invitations ?? [];

  const inviteMutation = useMutation({
    mutationFn: (data: { email: string; role: string }) => createInvitation(org.id, data),
    onSuccess: (data) => {
      showToast('success', 'Invitation created');
      setInviteResult(data);
      queryClient.invalidateQueries({ queryKey: ['org-invitations', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to create invitation', err.message),
  });

  const revokeMutation = useMutation({
    mutationFn: (nonce: string) => revokeInvitation(org.id, nonce),
    onSuccess: () => {
      showToast('success', 'Invitation revoked');
      queryClient.invalidateQueries({ queryKey: ['org-invitations', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to revoke invitation', err.message),
  });

  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button size="sm" onClick={() => { setShowCreate(true); setInviteResult(null); setInviteEmail(''); }}>
          <UserPlus className="w-4 h-4 mr-2" />
          Create Invitation
        </Button>
      </div>

      {isLoading ? (
        <div className="text-center py-8 text-muted-foreground">Loading invitations...</div>
      ) : invitations.length === 0 ? (
        <div className="text-center py-8 text-muted-foreground">
          <UserPlus className="w-8 h-8 mx-auto mb-3" />
          <p>No pending invitations.</p>
          <p className="text-sm mt-1">Invitations are single-use and expire after acceptance.</p>
        </div>
      ) : (
        <div className="space-y-2">
          {invitations.map((inv) => (
            <div
              key={inv.nonce}
              className="flex items-center justify-between border rounded-lg p-3"
            >
              <div className="min-w-0">
                <div className="font-medium truncate">{inv.email}</div>
                <div className="text-xs text-muted-foreground">
                  Role: <span className="uppercase tracking-wide">{inv.role}</span>
                  {' · '}
                  Created {new Date(inv.created_at).toLocaleString()}
                </div>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={() => revokeMutation.mutate(inv.nonce)}
                disabled={revokeMutation.isPending}
              >
                <Trash2 className="w-4 h-4 mr-1" />
                Revoke
              </Button>
            </div>
          ))}
        </div>
      )}

      <Dialog open={showCreate} onOpenChange={(open) => { setShowCreate(open); if (!open) setInviteResult(null); }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Create Invitation</DialogTitle>
            <DialogDescription>Generate a shareable invitation link for a new member</DialogDescription>
          </DialogHeader>
          {inviteResult ? (
            <div className="space-y-4">
              <div>
                <Label>Invitation Link</Label>
                <div className="flex gap-2 mt-1">
                  <Input readOnly value={`${window.location.origin}/invite/${inviteResult.token}`} />
                  <Button
                    variant="outline"
                    onClick={() => {
                      navigator.clipboard.writeText(`${window.location.origin}/invite/${inviteResult.token}`);
                      showToast('success', 'Copied to clipboard');
                    }}
                  >
                    <Copy className="w-4 h-4" />
                  </Button>
                </div>
              </div>
              <p className="text-sm text-muted-foreground">Share this link with the person you want to invite.</p>
            </div>
          ) : (
            <div className="space-y-4">
              <div>
                <Label>Email</Label>
                <Input
                  type="email"
                  placeholder="user@example.com"
                  value={inviteEmail}
                  onChange={(e) => setInviteEmail(e.target.value)}
                />
              </div>
              <div>
                <Label>Role</Label>
                <select
                  className="w-full border rounded px-3 py-2 bg-background mt-1"
                  value={inviteRole}
                  onChange={(e) => setInviteRole(e.target.value)}
                >
                  <option value="member">Member</option>
                  <option value="admin">Admin</option>
                </select>
              </div>
            </div>
          )}
          <DialogFooter>
            {inviteResult ? (
              <Button onClick={() => { setShowCreate(false); setInviteResult(null); setInviteEmail(''); }}>Done</Button>
            ) : (
              <>
                <Button variant="outline" onClick={() => setShowCreate(false)}>Cancel</Button>
                <Button
                  onClick={() => inviteMutation.mutate({ email: inviteEmail, role: inviteRole })}
                  disabled={!inviteEmail.trim() || inviteMutation.isPending}
                >
                  {inviteMutation.isPending ? 'Creating...' : 'Create Invitation'}
                </Button>
              </>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function UsageQuotaTab({ org }: { org: Organization }) {
  const { data: usage, isLoading: usageLoading } = useQuery({
    queryKey: ['org-usage', org.id],
    queryFn: () => fetchOrgUsage(org.id),
  });

  const { data: billing, isLoading: billingLoading } = useQuery({
    queryKey: ['org-billing', org.id],
    queryFn: () => fetchOrgBilling(org.id),
  });

  const { data: quota, isLoading: quotaLoading } = useQuery({
    queryKey: ['org-quota', org.id],
    queryFn: () => fetchOrgQuota(org.id),
  });

  const isLoading = usageLoading || billingLoading || quotaLoading;

  if (isLoading) {
    return <div className="text-center py-4 text-muted-foreground">Loading usage data...</div>;
  }

  return (
    <div className="space-y-6">
      {/* Usage Stats */}
      {usage && (
        <div>
          <h4 className="text-sm font-semibold mb-3">Current Usage</h4>
          <div className="grid gap-3 grid-cols-2">
            <div className="p-3 border rounded-lg">
              <div className="text-sm text-muted-foreground">Total Requests</div>
              <div className="text-2xl font-bold">{usage.total_requests.toLocaleString()}</div>
            </div>
            <div className="p-3 border rounded-lg">
              <div className="text-sm text-muted-foreground">Storage</div>
              <div className="text-2xl font-bold">{usage.total_storage_gb.toFixed(2)} GB</div>
            </div>
            <div className="p-3 border rounded-lg">
              <div className="text-sm text-muted-foreground">AI Tokens</div>
              <div className="text-2xl font-bold">{usage.total_ai_tokens.toLocaleString()}</div>
            </div>
            <div className="p-3 border rounded-lg">
              <div className="text-sm text-muted-foreground">Hosted Mocks</div>
              <div className="text-2xl font-bold">{usage.hosted_mocks_count}</div>
            </div>
            <div className="p-3 border rounded-lg">
              <div className="text-sm text-muted-foreground">Plugins Published</div>
              <div className="text-2xl font-bold">{usage.plugins_published}</div>
            </div>
            <div className="p-3 border rounded-lg">
              <div className="text-sm text-muted-foreground">API Tokens</div>
              <div className="text-2xl font-bold">{usage.api_tokens_count}</div>
            </div>
          </div>
        </div>
      )}

      {/* Billing Info */}
      {billing && (
        <div>
          <h4 className="text-sm font-semibold mb-3">Billing</h4>
          <div className="p-3 border rounded-lg space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-sm">Plan</span>
              {getPlanBadge(billing.plan)}
            </div>
            {billing.subscription && (
              <>
                <div className="flex items-center justify-between">
                  <span className="text-sm">Status</span>
                  <Badge className={billing.subscription.status === 'active' ? 'bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-300' : ''}>
                    {billing.subscription.status}
                  </Badge>
                </div>
                {billing.subscription.current_period_end && (
                  <div className="flex items-center justify-between">
                    <span className="text-sm">Period Ends</span>
                    <span className="text-sm text-muted-foreground">
                      {new Date(billing.subscription.current_period_end).toLocaleDateString()}
                    </span>
                  </div>
                )}
                {billing.subscription.cancel_at_period_end && (
                  <Badge className="bg-warning-100 text-warning-700 dark:bg-warning-900/30 dark:text-warning-300">
                    Cancels at period end
                  </Badge>
                )}
              </>
            )}
          </div>
        </div>
      )}

      {/* Quota */}
      {quota && Object.keys(quota.quota).length > 0 && (
        <div>
          <h4 className="text-sm font-semibold mb-3">Quota</h4>
          <div className="p-3 border rounded-lg space-y-2">
            {Object.entries(quota.quota).map(([key, value]) => {
              const meta = formatQuotaKey(key);
              return (
                <div key={key} className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">{meta.label}</span>
                  <span className="font-medium">{formatQuotaValue(value, meta.unit)}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

function AISettingsTab({ org }: { org: Organization }) {
  const { showToast } = useToast();
  const queryClient = useQueryClient();

  const { data: aiSettings, isLoading } = useQuery({
    queryKey: ['org-ai-settings', org.id],
    queryFn: () => fetchOrgAISettings(org.id),
  });

  const [maxDay, setMaxDay] = useState(0);
  const [maxMonth, setMaxMonth] = useState(0);
  const [flags, setFlags] = useState({
    ai_studio_enabled: true,
    ai_contract_diff_enabled: true,
    mockai_enabled: true,
    persona_generation_enabled: true,
  });

  useEffect(() => {
    if (aiSettings) {
      setMaxDay(aiSettings.max_ai_calls_per_workspace_per_day);
      setMaxMonth(aiSettings.max_ai_calls_per_workspace_per_month);
      setFlags(aiSettings.feature_flags);
    }
  }, [aiSettings]);

  const saveMutation = useMutation({
    mutationFn: () =>
      updateOrgAISettings(org.id, {
        max_ai_calls_per_workspace_per_day: maxDay,
        max_ai_calls_per_workspace_per_month: maxMonth,
        feature_flags: flags,
      }),
    onSuccess: () => {
      showToast('success', 'AI settings updated');
      queryClient.invalidateQueries({ queryKey: ['org-ai-settings', org.id] });
    },
    onError: (err: Error) => showToast('error', 'Failed to update AI settings', err.message),
  });

  if (isLoading) {
    return <div className="text-center py-4 text-muted-foreground">Loading AI settings...</div>;
  }

  return (
    <div className="space-y-6">
      <div className="space-y-4">
        <h4 className="text-sm font-semibold">Rate Limits</h4>
        <div>
          <Label>Max AI calls per workspace per day</Label>
          <Input type="number" value={maxDay} onChange={(e) => setMaxDay(Number(e.target.value))} />
        </div>
        <div>
          <Label>Max AI calls per workspace per month</Label>
          <Input type="number" value={maxMonth} onChange={(e) => setMaxMonth(Number(e.target.value))} />
        </div>
      </div>

      <div className="space-y-3">
        <h4 className="text-sm font-semibold">Feature Flags</h4>
        {([
          ['ai_studio_enabled', 'AI Studio'],
          ['ai_contract_diff_enabled', 'AI Contract Diff'],
          ['mockai_enabled', 'MockAI'],
          ['persona_generation_enabled', 'Persona Generation'],
        ] as const).map(([key, label]) => (
          <div key={key} className="flex items-center justify-between p-2 border rounded-lg">
            <span className="text-sm">{label}</span>
            <button
              onClick={() => setFlags((f) => ({ ...f, [key]: !f[key] }))}
              className="flex items-center gap-2 cursor-pointer"
            >
              {flags[key] ? (
                <ToggleRight className="w-6 h-6 text-primary" />
              ) : (
                <ToggleLeft className="w-6 h-6 text-muted-foreground" />
              )}
              <span className="text-sm">{flags[key] ? 'On' : 'Off'}</span>
            </button>
          </div>
        ))}
      </div>

      <Button onClick={() => saveMutation.mutate()} disabled={saveMutation.isPending}>
        <Save className="w-4 h-4 mr-2" />
        {saveMutation.isPending ? 'Saving...' : 'Save AI Settings'}
      </Button>
    </div>
  );
}

// ─── Main Page Component ─────────────────────────────────────────────────────

// Tab values that the OrganizationPage will honor via the `?tab=` URL param
// (e.g. when ApiTokensPage deep-links into the audit log).
const VALID_TABS = new Set([
  'members', 'settings', 'invitations', 'audit', 'templates',
  'sso', 'usage', 'ai', 'security',
]);

export function OrganizationPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
  const [selectedOrgId, setSelectedOrgId] = useState<string | null>(null);
  const [showCreateOrg, setShowCreateOrg] = useState(false);
  const [newOrgName, setNewOrgName] = useState('');
  const [newOrgSlug, setNewOrgSlug] = useState('');

  const tabParam = searchParams.get('tab');
  const activeTab = tabParam && VALID_TABS.has(tabParam) ? tabParam : 'members';
  const handleTabChange = (next: string) => {
    const params = new URLSearchParams(searchParams);
    if (next === 'members') {
      params.delete('tab');
    } else {
      params.set('tab', next);
    }
    // Don't preserve a stale event_type filter when the user manually
    // navigates away from the audit log tab.
    if (next !== 'audit') params.delete('event_type');
    setSearchParams(params, { replace: true });
  };

  const { data: organizations, isLoading: orgsLoading } = useQuery({
    queryKey: ['organizations'],
    queryFn: fetchOrganizations,
  });

  const createOrgMutation = useMutation({
    mutationFn: () => createOrganization({ name: newOrgName, slug: newOrgSlug }),
    onSuccess: (newOrg) => {
      showToast('success', 'Organization created', `${newOrg.name} has been created`);
      setShowCreateOrg(false);
      setNewOrgName('');
      setNewOrgSlug('');
      setSelectedOrgId(newOrg.id);
      queryClient.invalidateQueries({ queryKey: ['organizations'] });
    },
    onError: (err: Error) => showToast('error', 'Failed to create organization', err.message),
  });

  // Auto-generate slug from name
  const handleNameChange = (name: string) => {
    setNewOrgName(name);
    setNewOrgSlug(
      name
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-|-$/g, '')
    );
  };

  if (orgsLoading) {
    return (
      <div className="container mx-auto p-6">
        <div className="text-center py-12 text-muted-foreground">Loading organizations...</div>
      </div>
    );
  }

  const selectedOrg = organizations?.find((org) => org.id === selectedOrgId);

  return (
    <div className="mx-auto max-w-screen-2xl p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Organizations</h1>
        <p className="text-muted-foreground mt-2">
          Manage your organizations and team members
        </p>
      </div>

      <div className="grid gap-6 lg:grid-cols-[320px_1fr]">
        {/* Organizations List */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="flex items-center">
                <Building2 className="w-5 h-5 mr-2" />
                Your Organizations
              </CardTitle>
              <Button size="sm" onClick={() => setShowCreateOrg(true)}>
                <Plus className="w-4 h-4 mr-1" />
                New
              </Button>
            </div>
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
              <Tabs value={activeTab} onValueChange={handleTabChange} className="w-full">
                <TabsList className="flex w-full overflow-x-auto">
                  <TabsTrigger value="members">
                    <Users className="w-4 h-4 mr-1" />
                    Members
                  </TabsTrigger>
                  <TabsTrigger value="settings">
                    <Settings className="w-4 h-4 mr-1" />
                    Settings
                  </TabsTrigger>
                  <TabsTrigger value="invitations">
                    <UserPlus className="w-4 h-4 mr-1" />
                    Invitations
                  </TabsTrigger>
                  <TabsTrigger value="audit">
                    <ScrollText className="w-4 h-4 mr-1" />
                    Audit Log
                  </TabsTrigger>
                  <TabsTrigger value="templates">
                    <FileText className="w-4 h-4 mr-1" />
                    Templates
                  </TabsTrigger>
                  <TabsTrigger value="sso">
                    <Lock className="w-4 h-4 mr-1" />
                    SSO
                  </TabsTrigger>
                  <TabsTrigger value="usage">
                    <BarChart3 className="w-4 h-4 mr-1" />
                    Usage
                  </TabsTrigger>
                  <TabsTrigger value="ai">
                    <Brain className="w-4 h-4 mr-1" />
                    AI
                  </TabsTrigger>
                  <TabsTrigger value="security">
                    <AlertTriangle className="w-4 h-4 mr-1" />
                    Security
                  </TabsTrigger>
                </TabsList>

                <TabsContent value="members" className="mt-4">
                  <MembersTab org={selectedOrg} />
                </TabsContent>

                <TabsContent value="settings" className="mt-4">
                  <SettingsTab org={selectedOrg} />
                </TabsContent>

                <TabsContent value="invitations" className="mt-4">
                  <InvitationsTab org={selectedOrg} />
                </TabsContent>

                <TabsContent value="audit" className="mt-4">
                  <AuditLogTab org={selectedOrg} />
                </TabsContent>

                <TabsContent value="templates" className="mt-4">
                  <TemplatesTab org={selectedOrg} />
                </TabsContent>

                <TabsContent value="sso" className="mt-4">
                  <SSOTab org={selectedOrg} />
                </TabsContent>

                <TabsContent value="usage" className="mt-4">
                  <UsageQuotaTab org={selectedOrg} />
                </TabsContent>

                <TabsContent value="ai" className="mt-4">
                  <AISettingsTab org={selectedOrg} />
                </TabsContent>

                <TabsContent value="security" className="mt-4">
                  <SecurityActivityTab org={selectedOrg} />
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

      {/* Create Organization Dialog */}
      <Dialog open={showCreateOrg} onOpenChange={setShowCreateOrg}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Create Organization</DialogTitle>
            <DialogDescription>Create a new organization to collaborate with your team</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>Organization Name</Label>
              <Input
                value={newOrgName}
                onChange={(e) => handleNameChange(e.target.value)}
                placeholder="My Organization"
              />
            </div>
            <div>
              <Label>Slug</Label>
              <Input
                value={newOrgSlug}
                onChange={(e) => setNewOrgSlug(e.target.value)}
                placeholder="my-organization"
              />
              <p className="text-xs text-muted-foreground mt-1">
                URL-friendly identifier. Auto-generated from name.
              </p>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateOrg(false)}>Cancel</Button>
            <Button
              onClick={() => createOrgMutation.mutate()}
              disabled={!newOrgName.trim() || !newOrgSlug.trim() || createOrgMutation.isPending}
            >
              {createOrgMutation.isPending ? 'Creating...' : 'Create Organization'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
