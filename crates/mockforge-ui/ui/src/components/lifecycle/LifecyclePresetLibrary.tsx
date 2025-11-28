/**
 * Lifecycle Preset Library Component
 *
 * Displays all available lifecycle presets with their states, transitions,
 * and affected endpoints. Allows users to view preset details and apply
 * presets to personas.
 */

import React, { useState } from 'react';
import {
  BookOpen,
  ChevronRight,
  Clock,
  ArrowRight,
  CheckCircle2,
  Loader2,
  Info,
  Zap,
  Users,
  ShoppingCart,
  CreditCard,
  UserCheck
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/Card';
import { Button } from '../ui/button';
import { Badge } from '../ui/Badge';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger } from '../ui/Dialog';
import { Alert } from '../ui/DesignSystem';
import { toast } from 'sonner';
import {
  useLifecyclePresets,
  useLifecyclePresetDetails,
  useApplyLifecyclePreset
} from '../../hooks/useApi';
import { cn } from '../../utils/cn';

interface LifecyclePresetLibraryProps {
  className?: string;
  workspace?: string;
  activePersonaId?: string;
}

const PRESET_ICONS: Record<string, React.ReactNode> = {
  subscription: <CreditCard className="h-5 w-5" />,
  loan: <CreditCard className="h-5 w-5" />,
  order_fulfillment: <ShoppingCart className="h-5 w-5" />,
  user_engagement: <UserCheck className="h-5 w-5" />,
};

const PRESET_COLORS: Record<string, string> = {
  subscription: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
  loan: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
  order_fulfillment: 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200',
  user_engagement: 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200',
};

export function LifecyclePresetLibrary({
  className,
  workspace = 'default',
  activePersonaId
}: LifecyclePresetLibraryProps) {
  const { data: presetsData, isLoading: presetsLoading } = useLifecyclePresets();
  const applyMutation = useApplyLifecyclePreset();
  const [selectedPreset, setSelectedPreset] = useState<string | null>(null);
  const [detailsDialogOpen, setDetailsDialogOpen] = useState(false);
  const [applyDialogOpen, setApplyDialogOpen] = useState(false);

  const { data: presetDetails, isLoading: detailsLoading } = useLifecyclePresetDetails(
    selectedPreset || ''
  );

  const handleViewDetails = (presetId: string) => {
    setSelectedPreset(presetId);
    setDetailsDialogOpen(true);
  };

  const handleApplyPreset = (presetId: string) => {
    if (!activePersonaId) {
      toast.error('No active persona', {
        description: 'Please activate a persona before applying a lifecycle preset',
      });
      return;
    }

    setSelectedPreset(presetId);
    setApplyDialogOpen(true);
  };

  const confirmApply = () => {
    if (!selectedPreset || !activePersonaId) {
      return;
    }

    applyMutation.mutate(
      {
        workspace,
        personaId: activePersonaId,
        preset: selectedPreset,
      },
      {
        onSuccess: (data) => {
          toast.success('Lifecycle preset applied', {
            description: `Applied ${data.preset} to persona ${data.persona_id}. Current state: ${data.lifecycle_state}`,
          });
          setApplyDialogOpen(false);
          setSelectedPreset(null);
        },
        onError: (error) => {
          toast.error('Failed to apply preset', {
            description: error instanceof Error ? error.message : 'Unknown error',
          });
        },
      }
    );
  };

  if (presetsLoading) {
    return (
      <Card className={cn('p-6', className)}>
        <CardContent className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </CardContent>
      </Card>
    );
  }

  const presets = presetsData?.presets || [];

  return (
    <Card className={cn('p-6', className)}>
      <CardHeader>
        <CardTitle className="text-lg font-semibold flex items-center gap-2">
          <BookOpen className="h-5 w-5" />
          Lifecycle Preset Library
        </CardTitle>
        <CardDescription className="text-sm">
          Pre-configured lifecycle patterns for personas. Each preset defines states, transitions, and endpoint effects.
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-4">
        {presets.length === 0 ? (
          <Alert variant="info" className="mt-4">
            <Info className="h-4 w-4" />
            <div>
              <p className="font-medium">No presets available</p>
              <p className="text-sm text-muted-foreground">
                Lifecycle presets will appear here once they are configured.
              </p>
            </div>
          </Alert>
        ) : (
          <div className="grid gap-4 md:grid-cols-2">
            {presets.map((preset) => {
              const presetId = preset.id.toLowerCase();
              const icon = PRESET_ICONS[presetId] || <BookOpen className="h-5 w-5" />;
              const colorClass = PRESET_COLORS[presetId] || 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200';

              return (
                <Card key={preset.id} className="hover:shadow-md transition-shadow">
                  <CardHeader className="pb-3">
                    <div className="flex items-start justify-between">
                      <div className="flex items-center gap-3">
                        <div className={cn('p-2 rounded-lg', colorClass)}>
                          {icon}
                        </div>
                        <div>
                          <CardTitle className="text-base font-semibold">
                            {preset.name}
                          </CardTitle>
                          <CardDescription className="text-xs mt-1">
                            {preset.description}
                          </CardDescription>
                        </div>
                      </div>
                    </div>
                  </CardHeader>
                  <CardContent className="pt-0 space-y-3">
                    <div className="flex items-center gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleViewDetails(preset.id)}
                        className="flex-1"
                      >
                        <Info className="h-4 w-4 mr-2" />
                        View Details
                      </Button>
                      <Button
                        variant="default"
                        size="sm"
                        onClick={() => handleApplyPreset(preset.id)}
                        disabled={!activePersonaId || applyMutation.isPending}
                        className="flex-1"
                      >
                        {applyMutation.isPending ? (
                          <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                        ) : (
                          <Zap className="h-4 w-4 mr-2" />
                        )}
                        Apply
                      </Button>
                    </div>
                    {!activePersonaId && (
                      <p className="text-xs text-muted-foreground text-center">
                        Activate a persona to apply this preset
                      </p>
                    )}
                  </CardContent>
                </Card>
              );
            })}
          </div>
        )}

        {/* Details Dialog */}
        <Dialog open={detailsDialogOpen} onOpenChange={setDetailsDialogOpen}>
          <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
            <DialogHeader>
              <DialogTitle>
                {presetDetails?.preset.name || 'Preset Details'}
              </DialogTitle>
              <DialogDescription>
                {presetDetails?.preset.description}
              </DialogDescription>
            </DialogHeader>

            {detailsLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            ) : presetDetails ? (
              <div className="space-y-6 py-4">
                {/* Initial State */}
                <div>
                  <h4 className="text-sm font-semibold mb-2 flex items-center gap-2">
                    <Clock className="h-4 w-4" />
                    Initial State
                  </h4>
                  <Badge variant="outline" className="text-sm">
                    {presetDetails.initial_state}
                  </Badge>
                </div>

                {/* State Transitions */}
                <div>
                  <h4 className="text-sm font-semibold mb-3 flex items-center gap-2">
                    <ArrowRight className="h-4 w-4" />
                    State Transitions
                  </h4>
                  <div className="space-y-2">
                    {presetDetails.states.map((state, idx) => (
                      <div
                        key={idx}
                        className="flex items-center gap-3 p-3 border rounded-lg bg-muted/50"
                      >
                        <div className="flex-1">
                          <div className="flex items-center gap-2">
                            <Badge variant="secondary" className="text-xs">
                              {state.from}
                            </Badge>
                            <ArrowRight className="h-4 w-4 text-muted-foreground" />
                            <Badge variant="default" className="text-xs">
                              {state.to}
                            </Badge>
                          </div>
                          <div className="mt-2 text-xs text-muted-foreground space-y-1">
                            {state.after_days && (
                              <div className="flex items-center gap-1">
                                <Clock className="h-3 w-3" />
                                After {state.after_days} days
                              </div>
                            )}
                            {state.condition && (
                              <div className="font-mono text-xs bg-background px-2 py-1 rounded">
                                Condition: {state.condition}
                              </div>
                            )}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Affected Endpoints */}
                <div>
                  <h4 className="text-sm font-semibold mb-2 flex items-center gap-2">
                    <Zap className="h-4 w-4" />
                    Affected Endpoints
                  </h4>
                  <div className="flex flex-wrap gap-2">
                    {presetDetails.affected_endpoints.map((endpoint) => (
                      <Badge key={endpoint} variant="outline" className="text-xs">
                        {endpoint}
                      </Badge>
                    ))}
                  </div>
                </div>
              </div>
            ) : (
              <Alert variant="error">
                Failed to load preset details
              </Alert>
            )}

            <DialogFooter>
              <Button variant="outline" onClick={() => setDetailsDialogOpen(false)}>
                Close
              </Button>
              {activePersonaId && (
                <Button
                  onClick={() => {
                    setDetailsDialogOpen(false);
                    if (selectedPreset) {
                      handleApplyPreset(selectedPreset);
                    }
                  }}
                >
                  Apply to Persona
                </Button>
              )}
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {/* Apply Confirmation Dialog */}
        <Dialog open={applyDialogOpen} onOpenChange={setApplyDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Apply Lifecycle Preset</DialogTitle>
              <DialogDescription>
                This will apply the "{selectedPreset}" preset to persona "{activePersonaId}".
                The persona's lifecycle state will be set according to the preset's initial state.
              </DialogDescription>
            </DialogHeader>

            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => {
                  setApplyDialogOpen(false);
                  setSelectedPreset(null);
                }}
                disabled={applyMutation.isPending}
              >
                Cancel
              </Button>
              <Button
                onClick={confirmApply}
                disabled={applyMutation.isPending}
              >
                {applyMutation.isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    Applying...
                  </>
                ) : (
                  <>
                    <CheckCircle2 className="h-4 w-4 mr-2" />
                    Apply Preset
                  </>
                )}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </CardContent>
    </Card>
  );
}
