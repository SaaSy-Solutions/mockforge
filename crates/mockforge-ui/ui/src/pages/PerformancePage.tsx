/**
 * Performance Mode Page
 *
 * Main page for performance mode with:
 * - Load profile editor
 * - Performance metrics dashboard
 * - Bottleneck controls
 * - Start/stop controls
 */

import React from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '../components/ui/Card';
import { Button } from '../components/ui/button';
import { PerformanceMetrics } from '../components/performance/PerformanceMetrics';
import { LoadProfileEditor } from '../components/performance/LoadProfileEditor';
import { BottleneckControls } from '../components/performance/BottleneckControls';
import { usePerformanceStatus, useStartPerformance, useStopPerformance, useUpdateRps } from '../hooks/usePerformance';
import { Play, Square, Settings } from 'lucide-react';
import type { RpsProfile } from '@/hooks/usePerformance';

export default function PerformancePage() {
  const { data: status, isLoading: statusLoading } = usePerformanceStatus();
  const startPerformance = useStartPerformance();
  const stopPerformance = useStopPerformance();
  const updateRps = useUpdateRps();

  const handleStart = (profile: RpsProfile) => {
    startPerformance.mutate({
      initial_rps: profile.stages[0]?.target_rps || 10,
      rps_profile: profile,
    });
  };

  const handleStop = () => {
    if (confirm('Stop performance mode?')) {
      stopPerformance.mutate();
    }
  };

  const handleQuickStart = () => {
    startPerformance.mutate({
      initial_rps: 10,
    });
  };

  if (statusLoading) {
    return (
      <div className="container mx-auto p-6">
        <div className="animate-pulse">
          <div className="h-8 bg-gray-200 rounded w-1/4 mb-4"></div>
          <div className="h-64 bg-gray-200 rounded"></div>
        </div>
      </div>
    );
  }

  const isRunning = status?.running || false;

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold">Performance Mode</h1>
          <p className="text-muted-foreground mt-1">
            Lightweight load simulation with RPS control and bottleneck simulation
          </p>
        </div>
        <div className="flex gap-2">
          {!isRunning ? (
            <Button onClick={handleQuickStart} disabled={startPerformance.isPending}>
              <Play className="h-4 w-4 mr-2" />
              Quick Start
            </Button>
          ) : (
            <Button variant="destructive" onClick={handleStop} disabled={stopPerformance.isPending}>
              <Square className="h-4 w-4 mr-2" />
              Stop
            </Button>
          )}
        </div>
      </div>

      {/* Status Banner */}
      {isRunning && status && (
        <Card className="border-blue-200 bg-blue-50">
          <CardContent className="p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium">Performance mode is running</p>
                <p className="text-sm text-muted-foreground">
                  Target: {status.target_rps.toFixed(1)} RPS |
                  Current: {status.current_rps.toFixed(1)} RPS |
                  Bottlenecks: {status.bottlenecks}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <div className="h-3 w-3 bg-green-500 rounded-full animate-pulse"></div>
                <span className="text-sm font-medium">Active</span>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Metrics Dashboard */}
      {isRunning && <PerformanceMetrics />}

      {/* Main Content Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Load Profile Editor */}
        <LoadProfileEditor onStart={handleStart} initialRps={status?.target_rps || 10} />

        {/* Bottleneck Controls */}
        <BottleneckControls />
      </div>

      {/* Info Card */}
      <Card>
        <CardHeader>
          <CardTitle>About Performance Mode</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 text-sm text-muted-foreground">
          <p>
            Performance Mode is a lightweight load simulation tool designed to observe realistic behavior
            under stress testing conditions. It is <strong>not</strong> a true load testing tool, but rather
            a way to simulate bottlenecks and observe how your mocks respond to controlled load.
          </p>
          <ul className="list-disc list-inside space-y-1 ml-4">
            <li><strong>RPS Control:</strong> Maintain a target requests-per-second rate</li>
            <li><strong>Bottleneck Simulation:</strong> Simulate CPU, Memory, Network, I/O, and Database bottlenecks</li>
            <li><strong>Latency Recording:</strong> Track request latencies with detailed statistics</li>
            <li><strong>Real-time Metrics:</strong> Monitor performance metrics as they change</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
