/**
 * Network Profile Selector Component
 * 
 * Displays predefined and custom network profiles with one-click apply.
 * Shows profile preview and allows custom profile creation.
 */

import React, { useState } from 'react';
import {
  ModernCard,
  Section,
  ModernBadge,
} from '../ui/DesignSystem';
import { Button } from '../ui/button';
import {
  useNetworkProfiles,
  useApplyNetworkProfile,
  useCreateNetworkProfile,
  useDeleteNetworkProfile,
} from '../../hooks/useApi';
import { toast } from 'sonner';
import { Wifi, Zap, Plus, Trash2, Loader2 } from 'lucide-react';
import { Spinner } from '../ui/LoadingStates';

interface NetworkProfileSelectorProps {
  /** Callback when a profile is applied */
  onProfileApplied?: (profileName: string) => void;
}

export function NetworkProfileSelector({ onProfileApplied }: NetworkProfileSelectorProps) {
  const { data: profiles, isLoading, error } = useNetworkProfiles();
  const applyProfile = useApplyNetworkProfile();
  const createProfile = useCreateNetworkProfile();
  const deleteProfile = useDeleteNetworkProfile();
  const [showCreateModal, setShowCreateModal] = useState(false);

  const handleApply = async (profileName: string) => {
    try {
      await applyProfile.mutateAsync(profileName);
      toast.success(`Applied profile: ${profileName}`);
      onProfileApplied?.(profileName);
    } catch (error: any) {
      toast.error(`Failed to apply profile: ${error.message || 'Unknown error'}`);
    }
  };

  const handleDelete = async (profileName: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm(`Are you sure you want to delete the profile "${profileName}"?`)) {
      return;
    }

    try {
      await deleteProfile.mutateAsync(profileName);
      toast.success(`Deleted profile: ${profileName}`);
    } catch (error: any) {
      toast.error(`Failed to delete profile: ${error.message || 'Unknown error'}`);
    }
  };

  if (isLoading) {
    return (
      <Section title="Network Profiles" subtitle="Apply predefined or custom network conditions">
        <div className="flex items-center justify-center py-12">
          <Spinner size="lg" />
        </div>
      </Section>
    );
  }

  if (error) {
    return (
      <Section title="Network Profiles" subtitle="Apply predefined or custom network conditions">
        <ModernCard>
          <div className="text-center py-8">
            <p className="text-red-600 dark:text-red-400">
              Failed to load network profiles
            </p>
          </div>
        </ModernCard>
      </Section>
    );
  }

  const builtinProfiles = profiles?.filter((p) => p.builtin) || [];
  const customProfiles = profiles?.filter((p) => !p.builtin) || [];

  return (
    <Section
      title="Network Profiles"
      subtitle="One-click application of network conditions (slow 3G, flaky Wi-Fi, etc.)"
    >
      <div className="space-y-6">
        {/* Built-in Profiles */}
        {builtinProfiles.length > 0 && (
          <div>
            <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-4">
              Predefined Profiles
            </h3>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {builtinProfiles.map((profile) => (
                <ModernCard
                  key={profile.name}
                  className="hover:shadow-lg transition-shadow cursor-pointer"
                  onClick={() => handleApply(profile.name)}
                >
                  <div className="space-y-3">
                    <div className="flex items-start justify-between">
                      <div className="flex items-center gap-2">
                        <Wifi className="h-5 w-5 text-blue-500" />
                        <h4 className="font-semibold text-gray-900 dark:text-gray-100">
                          {profile.name.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase())}
                        </h4>
                      </div>
                      <ModernBadge variant="info" size="sm">
                        Built-in
                      </ModernBadge>
                    </div>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      {profile.description}
                    </p>
                    {profile.tags && profile.tags.length > 0 && (
                      <div className="flex flex-wrap gap-1">
                        {profile.tags.map((tag) => (
                          <span
                            key={tag}
                            className="px-2 py-0.5 text-xs bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 rounded"
                          >
                            {tag}
                          </span>
                        ))}
                      </div>
                    )}
                    <Button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleApply(profile.name);
                      }}
                      disabled={applyProfile.isPending}
                      className="w-full"
                      size="sm"
                    >
                      {applyProfile.isPending ? (
                        <>
                          <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                          Applying...
                        </>
                      ) : (
                        <>
                          <Zap className="h-4 w-4 mr-2" />
                          Apply Profile
                        </>
                      )}
                    </Button>
                  </div>
                </ModernCard>
              ))}
            </div>
          </div>
        )}

        {/* Custom Profiles */}
        <div>
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300">
              Custom Profiles
            </h3>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowCreateModal(true)}
            >
              <Plus className="h-4 w-4 mr-2" />
              Create Profile
            </Button>
          </div>
          {customProfiles.length > 0 ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {customProfiles.map((profile) => (
                <ModernCard
                  key={profile.name}
                  className="hover:shadow-lg transition-shadow"
                >
                  <div className="space-y-3">
                    <div className="flex items-start justify-between">
                      <div className="flex items-center gap-2">
                        <Wifi className="h-5 w-5 text-purple-500" />
                        <h4 className="font-semibold text-gray-900 dark:text-gray-100">
                          {profile.name}
                        </h4>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => handleDelete(profile.name, e)}
                        disabled={deleteProfile.isPending}
                        className="text-red-600 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      {profile.description}
                    </p>
                    {profile.tags && profile.tags.length > 0 && (
                      <div className="flex flex-wrap gap-1">
                        {profile.tags.map((tag) => (
                          <span
                            key={tag}
                            className="px-2 py-0.5 text-xs bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 rounded"
                          >
                            {tag}
                          </span>
                        ))}
                      </div>
                    )}
                    <Button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleApply(profile.name);
                      }}
                      disabled={applyProfile.isPending}
                      className="w-full"
                      size="sm"
                      variant="outline"
                    >
                      {applyProfile.isPending ? (
                        <>
                          <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                          Applying...
                        </>
                      ) : (
                        <>
                          <Zap className="h-4 w-4 mr-2" />
                          Apply Profile
                        </>
                      )}
                    </Button>
                  </div>
                </ModernCard>
              ))}
            </div>
          ) : (
            <ModernCard>
              <div className="text-center py-8">
                <Wifi className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                <p className="text-gray-600 dark:text-gray-400 mb-4">
                  No custom profiles yet
                </p>
                <Button
                  variant="outline"
                  onClick={() => setShowCreateModal(true)}
                >
                  <Plus className="h-4 w-4 mr-2" />
                  Create Your First Profile
                </Button>
              </div>
            </ModernCard>
          )}
        </div>
      </div>

      {/* Create Profile Modal - Simplified for now, can be enhanced later */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <ModernCard className="w-full max-w-2xl max-h-[90vh] overflow-y-auto">
            <div className="space-y-4">
              <h3 className="text-lg font-semibold">Create Custom Profile</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Custom profile creation will be available in a future update. For now, you can
                export the current chaos configuration as a profile template.
              </p>
              <div className="flex justify-end gap-3">
                <Button variant="outline" onClick={() => setShowCreateModal(false)}>
                  Close
                </Button>
              </div>
            </div>
          </ModernCard>
        </div>
      )}
    </Section>
  );
}

