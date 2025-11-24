import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Alert } from '@/components/ui/alert';
import { CheckCircle2, XCircle, AlertTriangle, Clock, Activity } from 'lucide-react';

interface ServiceStatus {
  name: string;
  status: 'operational' | 'degraded' | 'down';
  message?: string;
}

interface Incident {
  id: string;
  title: string;
  status: 'resolved' | 'investigating' | 'monitoring';
  started_at: string;
  resolved_at?: string;
  impact: 'minor' | 'major' | 'critical';
}

interface StatusResponse {
  status: 'operational' | 'degraded' | 'down';
  timestamp: string;
  services: ServiceStatus[];
  incidents: Incident[];
}

function StatusIcon({ status }: { status: string }) {
  switch (status) {
    case 'operational':
      return <CheckCircle2 className="h-5 w-5 text-green-500" />;
    case 'degraded':
      return <AlertTriangle className="h-5 w-5 text-yellow-500" />;
    case 'down':
      return <XCircle className="h-5 w-5 text-red-500" />;
    default:
      return <Clock className="h-5 w-5 text-gray-500" />;
  }
}

function StatusBadge({ status }: { status: string }) {
  const colors = {
    operational: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
    degraded: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
    down: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
  };

  return (
    <span
      className={`px-2 py-1 rounded-full text-xs font-medium ${
        colors[status as keyof typeof colors] || 'bg-gray-100 text-gray-800'
      }`}
    >
      {status.charAt(0).toUpperCase() + status.slice(1)}
    </span>
  );
}

export function StatusPage() {
  const { data: status, isLoading, error } = useQuery<StatusResponse>({
    queryKey: ['status'],
    queryFn: async () => {
      const response = await fetch('/api/v1/status');
      if (!response.ok) {
        throw new Error('Failed to fetch status');
      }
      return response.json();
    },
    refetchInterval: 60000, // Refresh every minute
  });

  if (isLoading) {
    return (
      <div className="container mx-auto px-4 py-8 max-w-4xl">
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="container mx-auto px-4 py-8 max-w-4xl">
        <Alert className="bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800">
          <span className="text-red-800 dark:text-red-200">
            Failed to load status information. Please try again later.
          </span>
        </Alert>
      </div>
    );
  }

  if (!status) {
    return null;
  }

  return (
    <div className="container mx-auto px-4 py-8 max-w-4xl">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Service Status</h1>
        <p className="text-muted-foreground">
          Real-time status of MockForge Cloud services
        </p>
      </div>

      {/* Overall Status */}
      <Card className="mb-6">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <StatusIcon status={status.status} />
              <CardTitle>All Systems {status.status === 'operational' ? 'Operational' : status.status === 'degraded' ? 'Degraded' : 'Down'}</CardTitle>
            </div>
            <StatusBadge status={status.status} />
          </div>
          <CardDescription>
            Last updated: {new Date(status.timestamp).toLocaleString()}
          </CardDescription>
        </CardHeader>
      </Card>

      {/* Services */}
      <Card className="mb-6">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            Services
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {status.services.map((service) => (
              <div
                key={service.name}
                className="flex items-center justify-between p-4 border rounded-lg"
              >
                <div className="flex items-center gap-3">
                  <StatusIcon status={service.status} />
                  <div>
                    <div className="font-medium">{service.name}</div>
                    {service.message && (
                      <div className="text-sm text-muted-foreground">
                        {service.message}
                      </div>
                    )}
                  </div>
                </div>
                <StatusBadge status={service.status} />
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* Incidents */}
      <Card>
        <CardHeader>
          <CardTitle>Recent Incidents</CardTitle>
        </CardHeader>
        <CardContent>
          {status.incidents.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <CheckCircle2 className="h-12 w-12 mx-auto mb-4 text-green-500" />
              <p>No incidents reported. All systems operational.</p>
            </div>
          ) : (
            <div className="space-y-4">
              {status.incidents.map((incident) => (
                <div
                  key={incident.id}
                  className="p-4 border rounded-lg"
                >
                  <div className="flex items-start justify-between mb-2">
                    <div>
                      <h3 className="font-medium">{incident.title}</h3>
                      <p className="text-sm text-muted-foreground">
                        Started: {new Date(incident.started_at).toLocaleString()}
                        {incident.resolved_at && (
                          <> • Resolved: {new Date(incident.resolved_at).toLocaleString()}</>
                        )}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <StatusBadge status={incident.status} />
                      <StatusBadge status={incident.impact} />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Footer Note */}
      <div className="mt-6 text-center text-sm text-muted-foreground">
        <p>
          Status page updates automatically every minute. For more information, visit{' '}
          <a
            href="https://docs.mockforge.dev"
            target="_blank"
            rel="noopener noreferrer"
            className="text-primary hover:underline"
          >
            our documentation
          </a>
          {' '}or{' '}
          <a
            href="/support"
            className="text-primary hover:underline"
          >
            contact support
          </a>
          .
        </p>
      </div>
    </div>
  );
}
