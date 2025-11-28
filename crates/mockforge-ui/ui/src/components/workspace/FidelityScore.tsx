import React, { useState, useEffect } from 'react';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../ui/Card';
import { Badge } from '../ui/Badge';
import { Alert } from '../ui/DesignSystem';
import { CheckCircle2, AlertTriangle, XCircle, Loader2 } from 'lucide-react';

interface FidelityScoreProps {
  workspaceId: string;
}

interface DriverMetric {
  value: number;
  percentage: number;
  label: string;
}

interface FidelityScoreData {
  overall: number;
  overall_percentage: number;
  driver_metrics: {
    schema_similarity: DriverMetric;
    sample_similarity: DriverMetric;
    response_time_similarity: DriverMetric;
    error_pattern_similarity: DriverMetric;
  };
  computed_at: string;
}

const FidelityScore: React.FC<FidelityScoreProps> = ({ workspaceId }) => {
  const [score, setScore] = useState<FidelityScoreData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchFidelityScore();
  }, [workspaceId]);

  const fetchFidelityScore = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch(
        `/api/v1/workspace/${workspaceId}/fidelity`
      );
      if (!response.ok) {
        if (response.status === 404) {
          setError('Fidelity score not yet calculated for this workspace');
        } else {
          throw new Error(`Failed to fetch fidelity score: ${response.statusText}`);
        }
        return;
      }
      const result = await response.json();
      if (result.success && result.score) {
        setScore(result.score);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  const getScoreColor = (percentage: number): string => {
    if (percentage >= 80) return '#4caf50'; // Green
    if (percentage >= 60) return '#ff9800'; // Orange
    return '#f44336'; // Red
  };

  const getScoreIcon = (percentage: number) => {
    if (percentage >= 80) return <CheckCircle2 className="h-6 w-6 text-green-500" />;
    if (percentage >= 60) return <AlertTriangle className="h-6 w-6 text-orange-500" />;
    return <XCircle className="h-6 w-6 text-red-500" />;
  };

  const getScoreLabel = (percentage: number): string => {
    if (percentage >= 80) return 'High Fidelity';
    if (percentage >= 60) return 'Medium Fidelity';
    return 'Low Fidelity';
  };

  const getScoreVariant = (percentage: number): 'default' | 'secondary' | 'destructive' => {
    if (percentage >= 80) return 'default';
    if (percentage >= 60) return 'secondary';
    return 'destructive';
  };

  if (loading) {
    return (
      <Card>
        <CardContent className="flex items-center justify-center min-h-[200px]">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <CardContent>
          <Alert variant="info" title="Fidelity Score">
            {error}
          </Alert>
        </CardContent>
      </Card>
    );
  }

  if (!score) {
    return null;
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Fidelity Score</CardTitle>
            <CardDescription>
              Last computed: {new Date(score.computed_at).toLocaleString()}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {/* Overall Score */}
        <div className="mb-6">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-3">
              <span className="text-4xl font-bold" style={{ color: getScoreColor(score.overall_percentage) }}>
                {score.overall_percentage}%
              </span>
              {getScoreIcon(score.overall_percentage)}
            </div>
            <Badge variant={getScoreVariant(score.overall_percentage)}>
              {getScoreLabel(score.overall_percentage)}
            </Badge>
          </div>
          <div className="w-full bg-secondary rounded-full h-3 mb-2">
            <div
              className="h-3 rounded-full transition-all"
              style={{
                width: `${score.overall_percentage}%`,
                backgroundColor: getScoreColor(score.overall_percentage),
              }}
            />
          </div>
          <p className="text-sm text-muted-foreground">
            Overall fidelity score: {score.overall.toFixed(3)} (0.0 = no match, 1.0 = perfect match)
          </p>
        </div>

        {/* Driver Metrics */}
        <div>
          <h3 className="text-lg font-semibold mb-4">Driver Metrics</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {/* Schema Similarity */}
            <div>
              <p className="text-sm text-muted-foreground mb-2">
                {score.driver_metrics.schema_similarity.label}
              </p>
              <p className="text-2xl font-bold mb-2" style={{ color: getScoreColor(score.driver_metrics.schema_similarity.percentage) }}>
                {score.driver_metrics.schema_similarity.percentage}%
              </p>
              <div className="w-full bg-secondary rounded-full h-2">
                <div
                  className="h-2 rounded-full transition-all"
                  style={{
                    width: `${score.driver_metrics.schema_similarity.percentage}%`,
                    backgroundColor: getScoreColor(score.driver_metrics.schema_similarity.percentage),
                  }}
                />
              </div>
            </div>

            {/* Sample Similarity */}
            <div>
              <p className="text-sm text-muted-foreground mb-2">
                {score.driver_metrics.sample_similarity.label}
              </p>
              <p className="text-2xl font-bold mb-2" style={{ color: getScoreColor(score.driver_metrics.sample_similarity.percentage) }}>
                {score.driver_metrics.sample_similarity.percentage}%
              </p>
              <div className="w-full bg-secondary rounded-full h-2">
                <div
                  className="h-2 rounded-full transition-all"
                  style={{
                    width: `${score.driver_metrics.sample_similarity.percentage}%`,
                    backgroundColor: getScoreColor(score.driver_metrics.sample_similarity.percentage),
                  }}
                />
              </div>
            </div>

            {/* Response Time Similarity */}
            <div>
              <p className="text-sm text-muted-foreground mb-2">
                {score.driver_metrics.response_time_similarity.label}
              </p>
              <p className="text-2xl font-bold mb-2" style={{ color: getScoreColor(score.driver_metrics.response_time_similarity.percentage) }}>
                {score.driver_metrics.response_time_similarity.percentage}%
              </p>
              <div className="w-full bg-secondary rounded-full h-2">
                <div
                  className="h-2 rounded-full transition-all"
                  style={{
                    width: `${score.driver_metrics.response_time_similarity.percentage}%`,
                    backgroundColor: getScoreColor(score.driver_metrics.response_time_similarity.percentage),
                  }}
                />
              </div>
            </div>

            {/* Error Pattern Similarity */}
            <div>
              <p className="text-sm text-muted-foreground mb-2">
                {score.driver_metrics.error_pattern_similarity.label}
              </p>
              <p className="text-2xl font-bold mb-2" style={{ color: getScoreColor(score.driver_metrics.error_pattern_similarity.percentage) }}>
                {score.driver_metrics.error_pattern_similarity.percentage}%
              </p>
              <div className="w-full bg-secondary rounded-full h-2">
                <div
                  className="h-2 rounded-full transition-all"
                  style={{
                    width: `${score.driver_metrics.error_pattern_similarity.percentage}%`,
                    backgroundColor: getScoreColor(score.driver_metrics.error_pattern_similarity.percentage),
                  }}
                />
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};

export default FidelityScore;
