import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { FederationDashboard } from '../components/federation/FederationDashboard';
import { Loader2, AlertCircle } from 'lucide-react';

interface Organization {
  id: string;
  name: string;
  slug: string;
  plan: string;
  owner_id: string;
  created_at: string;
}

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

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-12">
        <Loader2 className="w-8 h-8 animate-spin text-blue-600" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6 max-w-7xl mx-auto">
        <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300 p-4 rounded-lg flex items-center gap-3">
          <AlertCircle className="w-5 h-5 shrink-0" />
          <p>Failed to load organizations. Please try again later.</p>
        </div>
      </div>
    );
  }

  const orgId = organizations?.[0]?.id;

  if (!orgId) {
    return (
      <div className="p-6 max-w-7xl mx-auto">
        <div className="bg-yellow-50 dark:bg-yellow-900/20 text-yellow-700 dark:text-yellow-300 p-4 rounded-lg flex items-center gap-3">
          <AlertCircle className="w-5 h-5 shrink-0" />
          <p>No organization found. Please create an organization first.</p>
        </div>
      </div>
    );
  }

  return <FederationDashboard orgId={orgId} />;
};
