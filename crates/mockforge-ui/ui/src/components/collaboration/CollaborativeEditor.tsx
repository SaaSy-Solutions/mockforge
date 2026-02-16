/**
 * Collaborative Orchestration Editor
 *
 * Enables real-time collaborative editing of orchestrations with presence awareness,
 * conflict resolution, and change synchronization.
 */

import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  Box,
  Avatar,
  AvatarGroup,
  Chip,
  Tooltip,
  Badge,
  Typography,
  Alert,
  Snackbar,
} from '@mui/material';
import { useWebSocket } from '../../hooks/useWebSocket';

interface User {
  id: string;
  name: string;
  email: string;
  color: string;
  cursor?: { x: number; y: number };
  activeField?: string;
}

interface Change {
  id: string;
  userId: string;
  timestamp: Date;
  type: 'insert' | 'delete' | 'update';
  path: string;
  value: any;
  previousValue?: any;
}

interface CollaborativeEditorProps {
  orchestrationId: string;
  value: any;
  onChange: (value: any) => void;
  children: React.ReactNode;
}

export const CollaborativeEditor: React.FC<CollaborativeEditorProps> = ({
  orchestrationId,
  value,
  onChange,
  children,
}) => {
  const [activeUsers, setActiveUsers] = useState<User[]>([]);
  const [changes, setChanges] = useState<Change[]>([]);
  const [currentUser, setCurrentUser] = useState<User | null>(null);
  const [conflicts, setConflicts] = useState<string[]>([]);
  const [notification, setNotification] = useState<string | null>(null);
  const localChangesRef = useRef<Change[]>([]);
  const lastSyncedValueRef = useRef(value);

  const { messages, sendMessage, isConnected } = useWebSocket(
    `/api/collaboration/${orchestrationId}/ws`
  );

  // Initialize current user
  useEffect(() => {
    const initializeUser = async () => {
      const response = await fetch('/__mockforge/auth/me');
      if (response.ok) {
        const userData = await response.json();
        const user: User = {
          id: userData.id,
          name: userData.name,
          email: userData.email,
          color: generateUserColor(userData.id),
        };
        setCurrentUser(user);

        // Join collaboration session
        sendMessage({
          type: 'join',
          data: {
            orchestrationId,
            user,
          },
        });
      }
    };

    initializeUser();

    return () => {
      // Leave collaboration session
      sendMessage({
        type: 'leave',
        data: { orchestrationId },
      });
    };
  }, [orchestrationId]);

  // Handle incoming messages
  useEffect(() => {
    if (messages.length > 0) {
      const message = messages[messages.length - 1];
      handleMessage(message);
    }
  }, [messages]);

  const handleMessage = useCallback((message: any) => {
    switch (message.type) {
      case 'user_joined':
        setActiveUsers((prev) => [...prev, message.data.user]);
        setNotification(`${message.data.user.name} joined`);
        break;

      case 'user_left':
        setActiveUsers((prev) => prev.filter((u) => u.id !== message.data.userId));
        setNotification(`${message.data.userName} left`);
        break;

      case 'user_presence':
        setActiveUsers((prev) =>
          prev.map((u) =>
            u.id === message.data.userId
              ? { ...u, cursor: message.data.cursor, activeField: message.data.activeField }
              : u
          )
        );
        break;

      case 'change':
        handleRemoteChange(message.data.change);
        break;

      case 'sync':
        handleFullSync(message.data.value);
        break;

      case 'conflict':
        setConflicts((prev) => [...prev, message.data.message]);
        break;

      case 'users_list':
        setActiveUsers(message.data.users);
        break;
    }
  }, []);

  const handleRemoteChange = useCallback(
    (change: Change) => {
      // Skip if this is our own change
      if (currentUser && change.userId === currentUser.id) {
        return;
      }

      // Apply remote change
      const newValue = applyChange(value, change);
      onChange(newValue);
      lastSyncedValueRef.current = newValue;

      setChanges((prev) => [...prev, change]);

      // Check for conflicts with local changes
      const conflictingChanges = localChangesRef.current.filter(
        (local) => local.path === change.path
      );

      if (conflictingChanges.length > 0) {
        setConflicts((prev) => [
          ...prev,
          `Conflict detected in ${change.path}. Remote change applied.`,
        ]);
      }

      // Clear applied local changes
      localChangesRef.current = localChangesRef.current.filter(
        (local) => local.path !== change.path
      );
    },
    [value, onChange, currentUser]
  );

  const handleFullSync = useCallback(
    (syncedValue: any) => {
      onChange(syncedValue);
      lastSyncedValueRef.current = syncedValue;
      localChangesRef.current = [];
    },
    [onChange]
  );

  const handleLocalChange = useCallback(
    (path: string, newValue: any, previousValue: any) => {
      if (!currentUser) return;

      const change: Change = {
        id: generateChangeId(),
        userId: currentUser.id,
        timestamp: new Date(),
        type: previousValue === undefined ? 'insert' : 'update',
        path,
        value: newValue,
        previousValue,
      };

      localChangesRef.current.push(change);
      setChanges((prev) => [...prev, change]);

      // Send change to server
      sendMessage({
        type: 'change',
        data: {
          orchestrationId,
          change,
        },
      });
    },
    [currentUser, orchestrationId, sendMessage]
  );

  const handlePresenceUpdate = useCallback(
    (cursor: { x: number; y: number }, activeField?: string) => {
      if (!currentUser) return;

      sendMessage({
        type: 'presence',
        data: {
          orchestrationId,
          userId: currentUser.id,
          cursor,
          activeField,
        },
      });
    },
    [currentUser, orchestrationId, sendMessage]
  );

  return (
    <Box sx={{ position: 'relative' }}>
      {/* Active Users */}
      <Box
        sx={{
          position: 'absolute',
          top: 0,
          right: 0,
          zIndex: 1000,
          display: 'flex',
          gap: 1,
          p: 2,
        }}
      >
        {!isConnected && (
          <Chip label="Offline" color="error" size="small" sx={{ mr: 1 }} />
        )}
        <AvatarGroup max={5}>
          {activeUsers.map((user) => (
            <Tooltip key={user.id} title={`${user.name} - ${user.activeField || 'Viewing'}`}>
              <Avatar
                sx={{
                  bgcolor: user.color,
                  width: 32,
                  height: 32,
                  fontSize: '0.875rem',
                }}
              >
                {user.name.charAt(0).toUpperCase()}
              </Avatar>
            </Tooltip>
          ))}
        </AvatarGroup>
      </Box>

      {/* User Cursors */}
      {activeUsers.map((user) =>
        user.cursor ? (
          <Box
            key={user.id}
            sx={{
              position: 'absolute',
              left: user.cursor.x,
              top: user.cursor.y,
              pointerEvents: 'none',
              zIndex: 999,
            }}
          >
            <Box
              sx={{
                width: 20,
                height: 20,
                bgcolor: user.color,
                clipPath: 'polygon(0 0, 0 100%, 30% 70%, 45% 100%, 60% 65%, 100% 80%, 50% 50%)',
              }}
            />
            <Typography
              variant="caption"
              sx={{
                bgcolor: user.color,
                color: 'white',
                px: 1,
                py: 0.5,
                borderRadius: 1,
                ml: 2,
                whiteSpace: 'nowrap',
              }}
            >
              {user.name}
            </Typography>
          </Box>
        ) : null
      )}

      {/* Conflict Notifications */}
      {conflicts.length > 0 && (
        <Snackbar
          open={true}
          autoHideDuration={6000}
          onClose={() => setConflicts([])}
          anchorOrigin={{ vertical: 'top', horizontal: 'center' }}
        >
          <Alert severity="warning" onClose={() => setConflicts([])}>
            {conflicts[conflicts.length - 1]}
          </Alert>
        </Snackbar>
      )}

      {/* User Join/Leave Notifications */}
      {notification && (
        <Snackbar
          open={true}
          autoHideDuration={3000}
          onClose={() => setNotification(null)}
          anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
        >
          <Alert severity="info" onClose={() => setNotification(null)}>
            {notification}
          </Alert>
        </Snackbar>
      )}

      {/* Editor Content */}
      <Box
        onMouseMove={(e) => {
          handlePresenceUpdate({ x: e.clientX, y: e.clientY });
        }}
      >
        {React.cloneElement(children as React.ReactElement, {
          onChange: (newValue: any) => {
            const path = 'root'; // Simplified - in production, track specific paths
            handleLocalChange(path, newValue, value);
            onChange(newValue);
          },
        })}
      </Box>
    </Box>
  );
};

// Utility functions

function generateUserColor(userId: string): string {
  const colors = [
    '#FF6B6B',
    '#4ECDC4',
    '#45B7D1',
    '#FFA07A',
    '#98D8C8',
    '#F7DC6F',
    '#BB8FCE',
    '#85C1E2',
  ];
  const hash = userId.split('').reduce((acc, char) => acc + char.charCodeAt(0), 0);
  return colors[hash % colors.length];
}

function generateChangeId(): string {
  return `change_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
}

function applyChange(value: any, change: Change): any {
  // Simplified OT - in production, use proper CRDT or OT library
  const pathParts = change.path.split('.');
  const newValue = JSON.parse(JSON.stringify(value));

  let current = newValue;
  for (let i = 0; i < pathParts.length - 1; i++) {
    current = current[pathParts[i]];
  }

  const lastKey = pathParts[pathParts.length - 1];

  switch (change.type) {
    case 'insert':
    case 'update':
      current[lastKey] = change.value;
      break;
    case 'delete':
      delete current[lastKey];
      break;
  }

  return newValue;
}

export default CollaborativeEditor;
