import React, { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { FederationDashboard } from '../components/federation/FederationDashboard';
import { Loader2, AlertCircle, Building2 } from 'lucide-react';

interface Organization {
  id: string;
  name: string;
  slug: string;
  plan: string;
  owner_id: string;
  created_at: string;
}

const SELECTED_ORG_KEY = 'federation:selected-org-id';

async function fetchOrganizations(): Promise<Organization[]> {
  const token = localStorage.getItem('auth_token');
  const response = await fetch('/api/v1/organizations', {
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

export const FederationPage: React.FC = () => {
  const { data: organizations, isLoading, error } = useQuery({
    queryKey: ['organizations'],
    queryFn: fetchOrganizations,
  });

  // Persist the chosen org across navigation. Falls back to the first org
  // when nothing is stored yet, or when the stored org is no longer in the
  // user's org list (e.g. they were removed from it).
  const [selectedOrgId, setSelectedOrgIdState] = useState<string | null>(() => {
    try {
      return localStorage.getItem(SELECTED_ORG_KEY);
    } catch {
      return null;
    }
  });

  const setSelectedOrgId = (id: string) => {
    setSelectedOrgIdState(id);
    try {
      localStorage.setItem(SELECTED_ORG_KEY, id);
    } catch {
      // localStorage can throw in private mode / quota — selection still
      // works for the current session.
    }
  };

  // Reconcile the stored selection against the loaded org list.
  useEffect(() => {
    if (!organizations || organizations.length === 0) return;
    const stillAvailable =
      selectedOrgId && organizations.some((o) => o.id === selectedOrgId);
    if (!stillAvailable) {
      setSelectedOrgId(organizations[0].id);
    }
  }, [organizations, selectedOrgId]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-12">
        <Loader2 className="w-8 h-8 animate-spin text-info-600" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6 max-w-7xl mx-auto">
        <div className="bg-danger-50 dark:bg-danger-900/20 text-danger-700 dark:text-danger-300 p-4 rounded-lg flex items-center gap-3">
          <AlertCircle className="w-5 h-5 shrink-0" />
          <p>Failed to load organizations. Please try again later.</p>
        </div>
      </div>
    );
  }

  if (!organizations || organizations.length === 0) {
    return (
      <div className="p-6 max-w-7xl mx-auto">
        <div className="bg-warning-50 dark:bg-warning-900/20 text-warning-700 dark:text-warning-300 p-4 rounded-lg flex items-center gap-3">
          <AlertCircle className="w-5 h-5 shrink-0" />
          <p>No organization found. Please create an organization first.</p>
        </div>
      </div>
    );
  }

  const orgId = selectedOrgId ?? organizations[0].id;

  return (
    <div>
      {organizations.length > 1 && (
        <div className="px-6 pt-6">
          <label className="flex items-center gap-2 text-sm font-medium text-foreground">
            <Building2 className="h-4 w-4" />
            <span>Organization</span>
            <select
              value={orgId}
              onChange={(e) => setSelectedOrgId(e.target.value)}
              className="px-3 py-1.5 border border-border rounded-lg bg-card text-foreground text-sm"
            >
              {organizations.map((org) => (
                <option key={org.id} value={org.id}>
                  {org.name}
                </option>
              ))}
            </select>
          </label>
        </div>
      )}
      {/* Remount the dashboard when the selected org changes so its internal
          view-mode state (list/detail/edit/create) resets to the list view —
          a previously-selected federation in org A would otherwise be shown
          while the URL/list is now scoped to org B. */}
      <FederationDashboard key={orgId} orgId={orgId} />
    </div>
  );
};
