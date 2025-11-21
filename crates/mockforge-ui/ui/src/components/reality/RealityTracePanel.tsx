import React, { useEffect, useState } from 'react';
import { Info, User, Calendar, Zap, Activity, TrendingUp, ExternalLink } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/Card';
import { Badge } from '../ui/Badge';

/**
 * Reality Trace Metadata
 */
interface RealityTraceMetadata {
  reality_level?: {
    value: number;
    name: string;
  };
  reality_continuum_type: 'synthetic' | 'blended' | 'live';
  blend_ratio: number;
  data_source_breakdown: {
    recorded_percent: number;
    generator_percent: number;
    upstream_percent: number;
  };
  active_persona_id?: string;
  active_scenario?: string;
  active_chaos_profiles: string[];
  active_latency_profiles: string[];
}

interface RealityTracePanelProps {
  requestId?: string;
  className?: string;
  /**
   * Optional callback for navigation when clicking on deep-links
   * @param target - The target to navigate to ('persona' | 'scenario' | 'chaos')
   * @param id - The ID of the resource to navigate to
   */
  onNavigate?: (target: 'persona' | 'scenario' | 'chaos', id: string) => void;
}

/**
 * Reality Trace Panel Component
 *
 * Displays reality metadata for a request, showing:
 * - Reality level and continuum type
 * - Data source breakdown
 * - Active persona and scenario
 * - Active chaos and latency profiles
 */
export function RealityTracePanel({ requestId, className, onNavigate }: RealityTracePanelProps) {
  const [traceData, setTraceData] = useState<RealityTraceMetadata | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!requestId) {
      setTraceData(null);
      return;
    }

    const fetchTrace = async () => {
      setLoading(true);
      setError(null);
      try {
        const response = await fetch(`/__mockforge/api/reality/trace/${requestId}`);
        if (!response.ok) {
          throw new Error('Failed to fetch reality trace');
        }
        const data = await response.json();
        if (data.success && data.data) {
          setTraceData(data.data);
        } else {
          setTraceData(null);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load reality trace');
        setTraceData(null);
      } finally {
        setLoading(false);
      }
    };

    fetchTrace();
  }, [requestId]);

  if (!requestId) {
    return null;
  }

  if (loading) {
    return (
      <Card className={className}>
        <CardContent className="p-4">
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary"></div>
            <span className="ml-2 text-sm text-muted-foreground">Loading reality trace...</span>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className={className}>
        <CardContent className="p-4">
          <div className="text-sm text-destructive">{error}</div>
        </CardContent>
      </Card>
    );
  }

  if (!traceData) {
    return null;
  }

  const getContinuumTypeColor = (type: string) => {
    switch (type) {
      case 'synthetic':
        return 'bg-blue-500';
      case 'blended':
        return 'bg-purple-500';
      case 'live':
        return 'bg-green-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getContinuumTypeLabel = (type: string) => {
    switch (type) {
      case 'synthetic':
        return 'Synthetic';
      case 'blended':
        return 'Blended';
      case 'live':
        return 'Live';
      default:
        return type;
    }
  };

  return (
    <Card className={className}>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-lg">
          <Info className="h-5 w-5" />
          Reality Trace
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Reality Level and Continuum Type */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <div className="text-xs text-muted-foreground mb-1">Reality Level</div>
            {traceData.reality_level ? (
              <div className="flex items-center gap-2">
                <Badge variant="outline">{traceData.reality_level.value}</Badge>
                <span className="text-sm">{traceData.reality_level.name}</span>
              </div>
            ) : (
              <span className="text-sm text-muted-foreground">Not set</span>
            )}
          </div>
          <div>
            <div className="text-xs text-muted-foreground mb-1">Continuum Type</div>
            <Badge className={getContinuumTypeColor(traceData.reality_continuum_type)}>
              {getContinuumTypeLabel(traceData.reality_continuum_type)}
            </Badge>
          </div>
        </div>

        {/* Blend Ratio */}
        <div>
          <div className="flex items-center justify-between text-xs text-muted-foreground mb-1">
            <span>Blend Ratio</span>
            <span className="font-mono">{(traceData.blend_ratio * 100).toFixed(1)}%</span>
          </div>
          <div className="w-full bg-muted rounded-full h-2 overflow-hidden">
            <div
              className="bg-primary h-full transition-all"
              style={{ width: `${traceData.blend_ratio * 100}%` }}
            />
          </div>
          <div className="flex justify-between text-xs text-muted-foreground mt-1">
            <span>Mock</span>
            <span>Real</span>
          </div>
        </div>

        {/* Data Source Breakdown */}
        <div>
          <div className="text-xs text-muted-foreground mb-2 flex items-center gap-1">
            <TrendingUp className="h-3 w-3" />
            Data Source Breakdown
          </div>
          <div className="space-y-2">
            <div>
              <div className="flex items-center justify-between text-xs mb-1">
                <span>Recorded</span>
                <span className="font-mono">{traceData.data_source_breakdown.recorded_percent.toFixed(1)}%</span>
              </div>
              <div className="w-full bg-muted rounded-full h-1.5 overflow-hidden">
                <div
                  className="bg-blue-500 h-full transition-all"
                  style={{ width: `${traceData.data_source_breakdown.recorded_percent}%` }}
                />
              </div>
            </div>
            <div>
              <div className="flex items-center justify-between text-xs mb-1">
                <span>Generator</span>
                <span className="font-mono">{traceData.data_source_breakdown.generator_percent.toFixed(1)}%</span>
              </div>
              <div className="w-full bg-muted rounded-full h-1.5 overflow-hidden">
                <div
                  className="bg-purple-500 h-full transition-all"
                  style={{ width: `${traceData.data_source_breakdown.generator_percent}%` }}
                />
              </div>
            </div>
            <div>
              <div className="flex items-center justify-between text-xs mb-1">
                <span>Upstream</span>
                <span className="font-mono">{traceData.data_source_breakdown.upstream_percent.toFixed(1)}%</span>
              </div>
              <div className="w-full bg-muted rounded-full h-1.5 overflow-hidden">
                <div
                  className="bg-green-500 h-full transition-all"
                  style={{ width: `${traceData.data_source_breakdown.upstream_percent}%` }}
                />
              </div>
            </div>
          </div>
        </div>

        {/* Active Persona and Scenario */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <div className="text-xs text-muted-foreground mb-1 flex items-center gap-1">
              <User className="h-3 w-3" />
              Active Persona
            </div>
            {traceData.active_persona_id ? (
              <div
                onClick={() => onNavigate?.('persona', traceData.active_persona_id!)}
                className={`flex items-center gap-1 w-fit ${
                  onNavigate
                    ? 'cursor-pointer hover:opacity-80 transition-opacity'
                    : ''
                }`}
                title={onNavigate ? 'Click to view persona config' : undefined}
              >
                <Badge variant="outline" className="font-mono text-xs">
                  {traceData.active_persona_id}
                </Badge>
                {onNavigate && (
                  <ExternalLink className="h-3 w-3 text-muted-foreground" />
                )}
              </div>
            ) : (
              <span className="text-sm text-muted-foreground">None</span>
            )}
          </div>
          <div>
            <div className="text-xs text-muted-foreground mb-1 flex items-center gap-1">
              <Calendar className="h-3 w-3" />
              Active Scenario
            </div>
            {traceData.active_scenario ? (
              <div
                onClick={() => onNavigate?.('scenario', traceData.active_scenario!)}
                className={`flex items-center gap-1 w-fit ${
                  onNavigate
                    ? 'cursor-pointer hover:opacity-80 transition-opacity'
                    : ''
                }`}
                title={onNavigate ? 'Click to view scenario config' : undefined}
              >
                <Badge variant="outline" className="font-mono text-xs">
                  {traceData.active_scenario}
                </Badge>
                {onNavigate && (
                  <ExternalLink className="h-3 w-3 text-muted-foreground" />
                )}
              </div>
            ) : (
              <span className="text-sm text-muted-foreground">None</span>
            )}
          </div>
        </div>

        {/* Active Chaos Profiles */}
        {traceData.active_chaos_profiles.length > 0 && (
          <div>
            <div className="text-xs text-muted-foreground mb-2 flex items-center gap-1">
              <Zap className="h-3 w-3" />
              Active Chaos Profiles
            </div>
            <div className="flex flex-wrap gap-1">
              {traceData.active_chaos_profiles.map((profile, idx) => (
                <div
                  key={idx}
                  onClick={() => onNavigate?.('chaos', profile)}
                  className={`flex items-center gap-1 ${
                    onNavigate
                      ? 'cursor-pointer hover:opacity-80 transition-opacity'
                      : ''
                  }`}
                  title={onNavigate ? 'Click to view chaos profile config' : undefined}
                >
                  <Badge variant="destructive" className="text-xs">
                    {profile}
                  </Badge>
                  {onNavigate && (
                    <ExternalLink className="h-3 w-3 text-muted-foreground" />
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Active Latency Profiles */}
        {traceData.active_latency_profiles.length > 0 && (
          <div>
            <div className="text-xs text-muted-foreground mb-2 flex items-center gap-1">
              <Activity className="h-3 w-3" />
              Active Latency Profiles
            </div>
            <div className="flex flex-wrap gap-1">
              {traceData.active_latency_profiles.map((profile, idx) => (
                <Badge key={idx} variant="secondary" className="text-xs">
                  {profile}
                </Badge>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
