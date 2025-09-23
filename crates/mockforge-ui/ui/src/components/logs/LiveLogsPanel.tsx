import React, { useEffect, useRef, useState } from 'react';
import { LogEntry } from './LogEntry';
import { LogFilters } from './LogFilters';
import { LogDetails } from './LogDetails';
import { useLogStore } from '../../stores/useLogStore';

export function LiveLogsPanel() {
  const {
    filteredLogs,
    selectedLog,
    filter,
    autoScroll,
    isPaused,
    connectionStatus,
    selectLog,
    setFilter,
    clearFilter,
    setAutoScroll,
    setPaused,
    clearLogs,
  } = useLogStore();

  const [showDetails, setShowDetails] = useState(false);
  const logsEndRef = useRef<HTMLDivElement>(null);
  const logsContainerRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (autoScroll && !isPaused && logsEndRef.current) {
      logsEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [filteredLogs, autoScroll, isPaused]);

  const handleSelectLog = (log: typeof selectedLog) => {
    selectLog(log);
    setShowDetails(true);
  };

  const handleCloseDetails = () => {
    setShowDetails(false);
    selectLog(null);
  };

  const getConnectionStatusIndicator = () => {
    switch (connectionStatus) {
      case 'connected':
        return <div className="flex items-center space-x-2 text-green-600">
          <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
          <span className="text-sm">Connected</span>
        </div>;
      case 'connecting':
        return <div className="flex items-center space-x-2 text-yellow-600">
          <div className="w-2 h-2 bg-yellow-500 rounded-full animate-pulse" />
          <span className="text-sm">Connecting...</span>
        </div>;
      case 'disconnected':
        return <div className="flex items-center space-x-2 text-red-600">
          <div className="w-2 h-2 bg-red-500 rounded-full" />
          <span className="text-sm">Disconnected</span>
        </div>;
    }
  };

  const formatLogCount = () => {
    const total = filteredLogs.length;
    if (total === 0) return 'No logs';
    if (total === 1) return '1 log';
    return `${total.toLocaleString()} logs`;
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b">
        <div className="flex items-center space-x-4">
          <h2 className="text-2xl font-bold">Live Logs</h2>
          {getConnectionStatusIndicator()}
        </div>
        
        <div className="flex items-center space-x-4 text-sm text-muted-foreground">
          <span>{formatLogCount()}</span>
          <button 
            onClick={clearLogs}
            className="text-destructive hover:text-destructive/80"
          >
            Clear All
          </button>
        </div>
      </div>

      {/* Filters */}
      <LogFilters
        filter={filter}
        onFilterChange={setFilter}
        onClearFilters={clearFilter}
        autoScroll={autoScroll}
        onAutoScrollChange={setAutoScroll}
        isPaused={isPaused}
        onPauseChange={setPaused}
      />

      {/* Log Table Header */}
      <div className="flex items-center space-x-4 p-3 border-b bg-muted/50 font-mono text-xs font-semibold text-muted-foreground">
        <div className="w-20">Time</div>
        <div className="min-w-[60px]">Method</div>
        <div className="min-w-[50px]">Status</div>
        <div className="flex-1">Path</div>
        <div className="min-w-[60px]">Duration</div>
        <div className="min-w-[50px]">Size</div>
        <div className="min-w-[100px]">Client IP</div>
        <div className="w-8">Err</div>
      </div>

      {/* Logs List */}
      <div 
        ref={logsContainerRef}
        className="flex-1 overflow-auto"
      >
        {filteredLogs.length === 0 ? (
          <div className="flex items-center justify-center h-full text-center">
            <div className="space-y-4">
              <div className="text-6xl">📋</div>
              <div>
                <h3 className="text-lg font-semibold">No logs found</h3>
                <p className="text-muted-foreground">
                  {isPaused 
                    ? 'Logging is paused. Click "Live" to resume.'
                    : 'Waiting for log entries or adjust your filters.'
                  }
                </p>
              </div>
            </div>
          </div>
        ) : (
          <div>
            {filteredLogs.map((log) => (
              <LogEntry
                key={log.id}
                log={log}
                isSelected={selectedLog?.id === log.id}
                onSelect={handleSelectLog}
              />
            ))}
            <div ref={logsEndRef} />
          </div>
        )}
      </div>

      {/* Connection Status Bar */}
      <div className="p-2 border-t bg-muted/30">
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <div className="flex items-center space-x-4">
            {getConnectionStatusIndicator()}
            <span>•</span>
            <span>{formatLogCount()}</span>
            {filter.path_pattern && (
              <>
                <span>•</span>
                <span>Filtered by: "{filter.path_pattern}"</span>
              </>
            )}
          </div>
          <div className="flex items-center space-x-4">
            <span>Auto-scroll: {autoScroll ? 'On' : 'Off'}</span>
            <span>•</span>
            <span>Update rate: ~3s</span>
          </div>
        </div>
      </div>

      {/* Log Details Modal */}
      {showDetails && selectedLog && (
        <LogDetails log={selectedLog} onClose={handleCloseDetails} />
      )}
    </div>
  );
}