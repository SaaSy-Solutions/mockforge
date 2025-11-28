import { useState } from 'react';
import { Button } from '../ui/button';
import { useAuthStore } from '../../stores/useAuthStore';
import { AccountSettings } from './AccountSettings';
import { ProfileSettings } from './ProfileSettings';
import { Preferences } from './Preferences';
import { HelpSupport } from './HelpSupport';

export function UserProfile() {
  const { user, logout } = useAuthStore();
  const [showDropdown, setShowDropdown] = useState(false);
  const [showAccountSettings, setShowAccountSettings] = useState(false);
  const [showProfileSettings, setShowProfileSettings] = useState(false);
  const [showPreferences, setShowPreferences] = useState(false);
  const [showHelpSupport, setShowHelpSupport] = useState(false);

  if (!user) return null;

  const getRoleColor = (role: string) => {
    switch (role) {
      case 'admin':
        return 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300';
      case 'viewer':
        return 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300';
      default:
        return 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300';
    }
  };

  const getRoleIcon = (role: string) => {
    switch (role) {
      case 'admin':
        return '🔑';
      case 'viewer':
        return '👁️';
      default:
        return '👤';
    }
  };

  const handleLogout = () => {
    logout();
    setShowDropdown(false);
  };

  return (
    <div className="relative">
      <button
        onClick={() => setShowDropdown(!showDropdown)}
        className="flex items-center space-x-2 px-3 py-2 rounded-md hover:bg-accent transition-colors"
      >
        <div className="flex items-center space-x-2">
          <div className="w-8 h-8 bg-primary rounded-full flex items-center justify-center text-gray-900 dark:text-gray-100-foreground text-sm font-medium">
            {user.username.charAt(0).toUpperCase()}
          </div>
          <div className="text-left">
            <div className="text-sm font-medium">{user.username}</div>
            <div className={`text-xs px-2 py-0.5 rounded-full inline-flex items-center space-x-1 ${getRoleColor(user.role)}`}>
              <span>{getRoleIcon(user.role)}</span>
              <span>{user.role}</span>
            </div>
          </div>
        </div>
        <div className="text-muted-foreground">
          {showDropdown ? '▲' : '▼'}
        </div>
      </button>

      {showDropdown && (
        <>
          {/* Backdrop */}
          <div
            className="fixed inset-0 z-10"
            onClick={() => setShowDropdown(false)}
          />

          {/* Dropdown */}
          <div className="absolute right-0 mt-2 w-64 bg-card border rounded-md shadow-lg z-20">
            <div className="p-4 border-b">
              <div className="flex items-center space-x-3">
                <div className="w-10 h-10 bg-primary rounded-full flex items-center justify-center text-gray-900 dark:text-gray-100-foreground font-medium">
                  {user.username.charAt(0).toUpperCase()}
                </div>
                <div>
                  <div className="font-medium">{user.username}</div>
                  {user.email && (
                    <div className="text-sm text-muted-foreground">{user.email}</div>
                  )}
                  <div className={`text-xs px-2 py-0.5 rounded-full inline-flex items-center space-x-1 mt-1 ${getRoleColor(user.role)}`}>
                    <span>{getRoleIcon(user.role)}</span>
                    <span className="capitalize">{user.role}</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="p-2">
              <div className="space-y-1">
                <div className="px-3 py-2 text-xs text-muted-foreground">
                  Account
                </div>
                <button
                  className="w-full text-left px-3 py-2 text-sm hover:bg-accent rounded-md transition-colors"
                  onClick={() => {
                    setShowDropdown(false);
                    setShowAccountSettings(true);
                  }}
                >
                  Account Settings
                </button>
                <button
                  className="w-full text-left px-3 py-2 text-sm hover:bg-accent rounded-md transition-colors"
                  onClick={() => {
                    setShowDropdown(false);
                    setShowProfileSettings(true);
                  }}
                >
                  Profile Settings
                </button>
                <button
                  className="w-full text-left px-3 py-2 text-sm hover:bg-accent rounded-md transition-colors"
                  onClick={() => {
                    setShowDropdown(false);
                    setShowPreferences(true);
                  }}
                >
                  Preferences
                </button>
              </div>

              <div className="border-t my-2"></div>


              <div className="space-y-1">
                <div className="px-3 py-2 text-xs text-muted-foreground">
                  System
                </div>
                <a
                  href="https://docs.mockforge.dev/api/admin-ui-rest.html"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="w-full text-left px-3 py-2 text-sm hover:bg-accent rounded-md transition-colors block"
                  onClick={() => setShowDropdown(false)}
                >
                  API Documentation
                </a>
                <button
                  className="w-full text-left px-3 py-2 text-sm hover:bg-accent rounded-md transition-colors"
                  onClick={() => {
                    setShowDropdown(false);
                    setShowHelpSupport(true);
                  }}
                >
                  Help & Support
                </button>
              </div>

              <div className="border-t my-2"></div>

              <Button
                variant="ghost"
                className="w-full justify-start text-destructive hover:text-destructive hover:bg-destructive/10"
                onClick={handleLogout}
              >
                Sign Out
              </Button>
            </div>
          </div>
        </>
      )}

      <AccountSettings
        open={showAccountSettings}
        onOpenChange={setShowAccountSettings}
      />

      <ProfileSettings
        open={showProfileSettings}
        onOpenChange={setShowProfileSettings}
      />

      <Preferences
        open={showPreferences}
        onOpenChange={setShowPreferences}
      />

      <HelpSupport
        open={showHelpSupport}
        onOpenChange={setShowHelpSupport}
      />
    </div>
  );
}
