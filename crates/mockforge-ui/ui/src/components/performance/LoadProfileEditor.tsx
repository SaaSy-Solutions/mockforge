/**
 * Load Profile Editor
 *
 * Allows users to create and configure RPS profiles:
 * - Constant RPS
 * - Ramp-up profiles
 * - Spike profiles
 * - Custom multi-stage profiles
 */

import React, { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '../ui/Card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { Plus, Trash2, Play } from 'lucide-react';
import type { RpsProfile, RpsStage } from '../../hooks/usePerformance';

interface LoadProfileEditorProps {
  onStart: (profile: RpsProfile) => void;
  initialRps?: number;
}

type ProfileType = 'constant' | 'ramp-up' | 'spike' | 'custom';

export function LoadProfileEditor({ onStart, initialRps = 10 }: LoadProfileEditorProps) {
  const [profileType, setProfileType] = useState<ProfileType>('constant');
  const [constantRps, setConstantRps] = useState(initialRps);

  // Ramp-up settings
  const [rampStartRps, setRampStartRps] = useState(10);
  const [rampEndRps, setRampEndRps] = useState(100);
  const [rampDuration, setRampDuration] = useState(60);

  // Spike settings
  const [spikeBaseRps, setSpikeBaseRps] = useState(50);
  const [spikePeakRps, setSpikePeakRps] = useState(200);
  const [spikeDuration, setSpikeDuration] = useState(10);

  // Custom profile stages
  const [customStages, setCustomStages] = useState<RpsStage[]>([
    { duration_secs: 30, target_rps: 10, name: 'Warm-up' },
    { duration_secs: 60, target_rps: 50, name: 'Sustain' },
  ]);

  const handleStart = () => {
    let profile: RpsProfile;

    switch (profileType) {
      case 'constant':
        profile = {
          name: `Constant ${constantRps} RPS`,
          stages: [{ duration_secs: 0, target_rps: constantRps, name: 'Constant' }],
        };
        break;
      case 'ramp-up':
        profile = {
          name: `Ramp-up ${rampStartRps} -> ${rampEndRps} RPS`,
          stages: [
            { duration_secs: rampDuration, target_rps: rampStartRps, name: 'Ramp-up' },
            { duration_secs: 0, target_rps: rampEndRps, name: 'Sustain' },
          ],
        };
        break;
      case 'spike':
        profile = {
          name: `Spike ${spikeBaseRps} -> ${spikePeakRps} RPS`,
          stages: [
            { duration_secs: 30, target_rps: spikeBaseRps, name: 'Base' },
            { duration_secs: spikeDuration, target_rps: spikePeakRps, name: 'Spike' },
            { duration_secs: 30, target_rps: spikeBaseRps, name: 'Recovery' },
          ],
        };
        break;
      case 'custom':
        profile = {
          name: 'Custom Profile',
          stages: customStages,
        };
        break;
    }

    onStart(profile);
  };

  const addCustomStage = () => {
    setCustomStages([...customStages, { duration_secs: 30, target_rps: 10, name: `Stage ${customStages.length + 1}` }]);
  };

  const removeCustomStage = (index: number) => {
    setCustomStages(customStages.filter((_, i) => i !== index));
  };

  const updateCustomStage = (index: number, field: keyof RpsStage, value: number | string) => {
    const updated = [...customStages];
    updated[index] = { ...updated[index], [field]: value };
    setCustomStages(updated);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Load Profile</CardTitle>
        <CardDescription>Configure the request rate profile for performance testing</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label>Profile Type</Label>
          <Select value={profileType} onValueChange={(value) => setProfileType(value as ProfileType)}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="constant">Constant RPS</SelectItem>
              <SelectItem value="ramp-up">Ramp-up</SelectItem>
              <SelectItem value="spike">Spike</SelectItem>
              <SelectItem value="custom">Custom</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {/* Constant RPS */}
        {profileType === 'constant' && (
          <div className="space-y-2">
            <Label>Target RPS</Label>
            <Input
              type="number"
              value={constantRps}
              onChange={(e) => setConstantRps(Number(e.target.value))}
              min="1"
            />
          </div>
        )}

        {/* Ramp-up */}
        {profileType === 'ramp-up' && (
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Start RPS</Label>
                <Input
                  type="number"
                  value={rampStartRps}
                  onChange={(e) => setRampStartRps(Number(e.target.value))}
                  min="1"
                />
              </div>
              <div className="space-y-2">
                <Label>End RPS</Label>
                <Input
                  type="number"
                  value={rampEndRps}
                  onChange={(e) => setRampEndRps(Number(e.target.value))}
                  min="1"
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label>Duration (seconds)</Label>
              <Input
                type="number"
                value={rampDuration}
                onChange={(e) => setRampDuration(Number(e.target.value))}
                min="1"
              />
            </div>
          </div>
        )}

        {/* Spike */}
        {profileType === 'spike' && (
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>Base RPS</Label>
                <Input
                  type="number"
                  value={spikeBaseRps}
                  onChange={(e) => setSpikeBaseRps(Number(e.target.value))}
                  min="1"
                />
              </div>
              <div className="space-y-2">
                <Label>Peak RPS</Label>
                <Input
                  type="number"
                  value={spikePeakRps}
                  onChange={(e) => setSpikePeakRps(Number(e.target.value))}
                  min="1"
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label>Spike Duration (seconds)</Label>
              <Input
                type="number"
                value={spikeDuration}
                onChange={(e) => setSpikeDuration(Number(e.target.value))}
                min="1"
              />
            </div>
          </div>
        )}

        {/* Custom */}
        {profileType === 'custom' && (
          <div className="space-y-4">
            <div className="flex justify-between items-center">
              <Label>Stages</Label>
              <Button type="button" variant="outline" size="sm" onClick={addCustomStage}>
                <Plus className="h-4 w-4 mr-2" />
                Add Stage
              </Button>
            </div>
            <div className="space-y-2">
              {customStages.map((stage, index) => (
                <div key={index} className="flex gap-2 items-end p-3 border rounded">
                  <div className="flex-1 space-y-2">
                    <Label>Stage Name</Label>
                    <Input
                      value={stage.name || ''}
                      onChange={(e) => updateCustomStage(index, 'name', e.target.value)}
                      placeholder="Stage name"
                    />
                  </div>
                  <div className="flex-1 space-y-2">
                    <Label>Duration (sec)</Label>
                    <Input
                      type="number"
                      value={stage.duration_secs}
                      onChange={(e) => updateCustomStage(index, 'duration_secs', Number(e.target.value))}
                      min="0"
                    />
                  </div>
                  <div className="flex-1 space-y-2">
                    <Label>Target RPS</Label>
                    <Input
                      type="number"
                      value={stage.target_rps}
                      onChange={(e) => updateCustomStage(index, 'target_rps', Number(e.target.value))}
                      min="1"
                    />
                  </div>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => removeCustomStage(index)}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
            </div>
          </div>
        )}

        <Button onClick={handleStart} className="w-full">
          <Play className="h-4 w-4 mr-2" />
          Start Performance Mode
        </Button>
      </CardContent>
    </Card>
  );
}
