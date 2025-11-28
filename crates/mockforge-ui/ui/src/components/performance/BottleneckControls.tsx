/**
 * Bottleneck Controls
 *
 * Allows users to add and manage bottlenecks:
 * - CPU, Memory, Network, I/O, Database bottlenecks
 * - Configure severity and endpoint patterns
 */

import React, { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '../ui/Card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select';
import { Slider } from '../ui/slider';
import { useAddBottleneck, useClearBottlenecks, usePerformanceStatus } from '../../hooks/usePerformance';
import { Trash2, Plus } from 'lucide-react';
import type { BottleneckConfig } from '../../hooks/usePerformance';

export function BottleneckControls() {
  const { data: status } = usePerformanceStatus();
  const addBottleneck = useAddBottleneck();
  const clearBottlenecks = useClearBottlenecks();

  const [bottleneckType, setBottleneckType] = useState<BottleneckConfig['bottleneck_type']>('network');
  const [severity, setSeverity] = useState(0.5);
  const [endpointPattern, setEndpointPattern] = useState('');
  const [duration, setDuration] = useState('');

  const handleAddBottleneck = () => {
    const config: BottleneckConfig = {
      bottleneck_type: bottleneckType,
      severity: severity,
      endpoint_pattern: endpointPattern || undefined,
      duration_secs: duration ? Number(duration) : undefined,
    };

    addBottleneck.mutate({ bottleneck: config });

    // Reset form
    setSeverity(0.5);
    setEndpointPattern('');
    setDuration('');
  };

  const handleClearBottlenecks = () => {
    if (confirm('Clear all bottlenecks?')) {
      clearBottlenecks.mutate();
    }
  };

  if (!status?.running) {
    return (
      <Card>
        <CardContent className="p-6">
          <div className="text-center text-muted-foreground">
            Start performance mode to configure bottlenecks
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex justify-between items-center">
          <div>
            <CardTitle>Bottlenecks</CardTitle>
            <CardDescription>Simulate system bottlenecks to observe behavior under stress</CardDescription>
          </div>
          {status.bottlenecks > 0 && (
            <Button variant="outline" size="sm" onClick={handleClearBottlenecks}>
              <Trash2 className="h-4 w-4 mr-2" />
              Clear All ({status.bottlenecks})
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label>Bottleneck Type</Label>
          <Select value={bottleneckType} onValueChange={(value) => setBottleneckType(value as BottleneckConfig['bottleneck_type'])}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="cpu">CPU</SelectItem>
              <SelectItem value="memory">Memory</SelectItem>
              <SelectItem value="network">Network</SelectItem>
              <SelectItem value="io">I/O</SelectItem>
              <SelectItem value="database">Database</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-2">
          <Label>Severity: {(severity * 100).toFixed(0)}%</Label>
          <Slider
            min={0}
            max={1}
            step={0.1}
            value={severity}
            onChange={setSeverity}
          />
          <p className="text-xs text-muted-foreground">
            {severity === 0 && 'No bottleneck'}
            {severity > 0 && severity < 0.3 && 'Light bottleneck'}
            {severity >= 0.3 && severity < 0.7 && 'Moderate bottleneck'}
            {severity >= 0.7 && 'Severe bottleneck'}
          </p>
        </div>

        <div className="space-y-2">
          <Label>Endpoint Pattern (optional)</Label>
          <Input
            value={endpointPattern}
            onChange={(e) => setEndpointPattern(e.target.value)}
            placeholder="/api/users"
          />
          <p className="text-xs text-muted-foreground">
            Leave empty to apply to all endpoints
          </p>
        </div>

        <div className="space-y-2">
          <Label>Duration (seconds, optional)</Label>
          <Input
            type="number"
            value={duration}
            onChange={(e) => setDuration(e.target.value)}
            placeholder="Leave empty for indefinite"
            min="1"
          />
        </div>

        <Button onClick={handleAddBottleneck} className="w-full" disabled={addBottleneck.isPending}>
          <Plus className="h-4 w-4 mr-2" />
          Add Bottleneck
        </Button>

        {status.bottleneck_types.length > 0 && (
          <div className="mt-4 p-3 bg-muted rounded">
            <p className="text-sm font-medium mb-2">Active Bottlenecks:</p>
            <ul className="text-sm text-muted-foreground space-y-1">
              {status.bottleneck_types.map((type, i) => (
                <li key={i} className="capitalize">{type}</li>
              ))}
            </ul>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
