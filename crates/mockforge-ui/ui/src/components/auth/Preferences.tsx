import { useState } from 'react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Switch } from '../ui/switch';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from '../ui/Dialog';
import {
  Tabs,
  TabsProvider,
  TabsList,
  TabsTrigger,
  TabsContent,
} from '../ui/Tabs';
import { usePreferencesStore } from '../../stores/usePreferencesStore';
import { useThemeStore } from '../../stores/useThemeStore';
import {
  Palette,
  FileText,
  Bell,
  Search,
  Settings,
  RotateCcw,
  Monitor,
  Sun,
  Moon
} from 'lucide-react';

interface PreferencesProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function Preferences({ open, onOpenChange }: PreferencesProps) {
  const [activeTab, setActiveTab] = useState('theme');
  const {
    preferences,
    updateTheme,
    updateLogs,
    updateNotifications,
    updateSearch,
    updateUI,
    resetToDefaults,
    savePreferences,
    loading,
    error,
  } = usePreferencesStore();

  const { setTheme: setThemeStore } = useThemeStore();

  const handleSave = async () => {
    try {
      await savePreferences();
      onOpenChange(false);
    } catch (error) {
      console.error('Failed to save preferences:', error);
    }
  };

  const handleReset = () => {
    resetToDefaults();
    // Also reset theme in the theme store
    setThemeStore('system');
  };

  const themeOptions = [
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'system', label: 'System', icon: Monitor },
  ];

  const accentColors = [
    { value: 'blue', label: 'Blue' },
    { value: 'green', label: 'Green' },
    { value: 'purple', label: 'Purple' },
    { value: 'orange', label: 'Orange' },
    { value: 'red', label: 'Red' },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Settings className="h-5 w-5" />
            Preferences
          </DialogTitle>
          <DialogDescription>
            Customize your experience with MockForge
          </DialogDescription>
          <DialogClose onClick={() => onOpenChange(false)} />
        </DialogHeader>

        <TabsProvider value={activeTab} onValueChange={setActiveTab}>
          <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
            <TabsList className="grid w-full grid-cols-5">
              <TabsTrigger value="theme" className="flex items-center gap-2">
                <Palette className="h-4 w-4" />
                <span className="hidden sm:inline">Theme</span>
              </TabsTrigger>
              <TabsTrigger value="logs" className="flex items-center gap-2">
                <FileText className="h-4 w-4" />
                <span className="hidden sm:inline">Logs</span>
              </TabsTrigger>
              <TabsTrigger value="notifications" className="flex items-center gap-2">
                <Bell className="h-4 w-4" />
                <span className="hidden sm:inline">Notifications</span>
              </TabsTrigger>
              <TabsTrigger value="search" className="flex items-center gap-2">
                <Search className="h-4 w-4" />
                <span className="hidden sm:inline">Search</span>
              </TabsTrigger>
              <TabsTrigger value="ui" className="flex items-center gap-2">
                <Settings className="h-4 w-4" />
                <span className="hidden sm:inline">UI</span>
              </TabsTrigger>
            </TabsList>

            <TabsContent value="theme" className="space-y-6">
              <div className="space-y-4">
                <div>
                  <label className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3 block">
                    Theme Mode
                  </label>
                  <div className="grid grid-cols-3 gap-2">
                    {themeOptions.map(({ value, label, icon: Icon }) => (
                      <button
                        key={value}
                        onClick={() => {
                          updateTheme({ theme: value as 'light' | 'dark' | 'system' });
                          setThemeStore(value as 'light' | 'dark' | 'system');
                        }}
                        className={`flex items-center gap-2 p-3 rounded-lg border transition-all ${
                          preferences.theme.theme === value
                            ? 'border-brand bg-brand/10 text-brand'
                            : 'border-border hover:border-brand/50'
                        }`}
                      >
                        <Icon className="h-4 w-4" />
                        <span className="text-sm">{label}</span>
                      </button>
                    ))}
                  </div>
                </div>

                <div>
                  <label className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3 block">
                    Accent Color
                  </label>
                  <div className="flex gap-2">
                    {accentColors.map(({ value, label }) => (
                      <button
                        key={value}
                        onClick={() => updateTheme({ accentColor: value as string })}
                        className={`w-8 h-8 rounded-full border-2 transition-all ${
                          preferences.theme.accentColor === value
                            ? 'border-gray-900 dark:border-gray-100 scale-110'
                            : 'border-gray-300 dark:border-gray-600'
                        }`}
                        style={{ backgroundColor: value }}
                        title={label}
                      />
                    ))}
                  </div>
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      High Contrast
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Increase contrast for better accessibility
                    </p>
                  </div>
                  <Switch
                    checked={preferences.theme.highContrast}
                    onCheckedChange={(checked) => updateTheme({ highContrast: checked })}
                  />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="logs" className="space-y-6">
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Auto-scroll
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Automatically scroll to new log entries
                    </p>
                  </div>
                  <Switch
                    checked={preferences.logs.autoScroll}
                    onCheckedChange={(checked) => updateLogs({ autoScroll: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Pause on Error
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Pause log streaming when errors occur
                    </p>
                  </div>
                  <Switch
                    checked={preferences.logs.pauseOnError}
                    onCheckedChange={(checked) => updateLogs({ pauseOnError: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Show Timestamps
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Display timestamps in log entries
                    </p>
                  </div>
                  <Switch
                    checked={preferences.logs.showTimestamps}
                    onCheckedChange={(checked) => updateLogs({ showTimestamps: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Compact View
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Use compact layout for log entries
                    </p>
                  </div>
                  <Switch
                    checked={preferences.logs.compactView}
                    onCheckedChange={(checked) => updateLogs({ compactView: checked })}
                  />
                </div>

                <div>
                  <label className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 block">
                    Default Time Range (hours)
                  </label>
                  <Input
                    type="number"
                    min="1"
                    max="168"
                    value={preferences.logs.defaultTimeRange}
                    onChange={(e) => updateLogs({ defaultTimeRange: parseInt(e.target.value) || 24 })}
                    className="w-24"
                  />
                </div>

                <div>
                  <label className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 block">
                    Items Per Page
                  </label>
                  <Input
                    type="number"
                    min="10"
                    max="1000"
                    step="10"
                    value={preferences.logs.itemsPerPage}
                    onChange={(e) => updateLogs({ itemsPerPage: parseInt(e.target.value) || 100 })}
                    className="w-24"
                  />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="notifications" className="space-y-6">
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Enable Sounds
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Play notification sounds
                    </p>
                  </div>
                  <Switch
                    checked={preferences.notifications.enableSounds}
                    onCheckedChange={(checked) => updateNotifications({ enableSounds: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Show Toasts
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Display toast notifications
                    </p>
                  </div>
                  <Switch
                    checked={preferences.notifications.showToasts}
                    onCheckedChange={(checked) => updateNotifications({ showToasts: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Notify on Errors
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Show notifications for error events
                    </p>
                  </div>
                  <Switch
                    checked={preferences.notifications.notifyOnErrors}
                    onCheckedChange={(checked) => updateNotifications({ notifyOnErrors: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Notify on Success
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Show notifications for successful operations
                    </p>
                  </div>
                  <Switch
                    checked={preferences.notifications.notifyOnSuccess}
                    onCheckedChange={(checked) => updateNotifications({ notifyOnSuccess: checked })}
                  />
                </div>

                <div>
                  <label className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 block">
                    Toast Duration (seconds)
                  </label>
                  <Input
                    type="number"
                    min="1"
                    max="30"
                    value={preferences.notifications.toastDuration}
                    onChange={(e) => updateNotifications({ toastDuration: parseInt(e.target.value) || 5 })}
                    className="w-24"
                  />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="search" className="space-y-6">
              <div className="space-y-4">
                <div>
                  <label className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3 block">
                    Default Search Scope
                  </label>
                  <div className="grid grid-cols-2 gap-2">
                    {[
                      { value: 'all', label: 'All' },
                      { value: 'current', label: 'Current Page' },
                      { value: 'logs', label: 'Logs Only' },
                      { value: 'services', label: 'Services Only' },
                    ].map(({ value, label }) => (
                      <button
                        key={value}
                        onClick={() => updateSearch({ defaultScope: value as 'current' | 'logs' | 'all' | 'services' })}
                        className={`p-2 text-sm rounded border transition-all ${
                          preferences.search.defaultScope === value
                            ? 'border-brand bg-brand/10 text-brand'
                            : 'border-border hover:border-brand/50'
                        }`}
                      >
                        {label}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Case Sensitive
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Match case in search queries
                    </p>
                  </div>
                  <Switch
                    checked={preferences.search.caseSensitive}
                    onCheckedChange={(checked) => updateSearch({ caseSensitive: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Regex Enabled
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Allow regular expressions in search
                    </p>
                  </div>
                  <Switch
                    checked={preferences.search.regexEnabled}
                    onCheckedChange={(checked) => updateSearch({ regexEnabled: checked })}
                  />
                </div>

                <div>
                  <label className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 block">
                    Max History Items
                  </label>
                  <Input
                    type="number"
                    min="5"
                    max="50"
                    value={preferences.search.maxHistoryItems}
                    onChange={(e) => updateSearch({ maxHistoryItems: parseInt(e.target.value) || 10 })}
                    className="w-24"
                  />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="ui" className="space-y-6">
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Sidebar Collapsed
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Start with collapsed sidebar
                    </p>
                  </div>
                  <Switch
                    checked={preferences.ui.sidebarCollapsed}
                    onCheckedChange={(checked) => updateUI({ sidebarCollapsed: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Confirm Delete
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Show confirmation dialogs for delete actions
                    </p>
                  </div>
                  <Switch
                    checked={preferences.ui.confirmDelete}
                    onCheckedChange={(checked) => updateUI({ confirmDelete: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Auto-save
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Automatically save changes
                    </p>
                  </div>
                  <Switch
                    checked={preferences.ui.autoSave}
                    onCheckedChange={(checked) => updateUI({ autoSave: checked })}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-900 dark:text-gray-100">
                      Keyboard Shortcuts
                    </label>
                    <p className="text-xs text-gray-600 dark:text-gray-400">
                      Enable keyboard shortcuts
                    </p>
                  </div>
                  <Switch
                    checked={preferences.ui.keyboardShortcuts}
                    onCheckedChange={(checked) => updateUI({ keyboardShortcuts: checked })}
                  />
                </div>

                <div>
                  <label className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-2 block">
                    Default Page
                  </label>
                  <select
                    value={preferences.ui.defaultPage}
                    onChange={(e) => updateUI({ defaultPage: e.target.value })}
                    className="w-full p-2 border border-border rounded-md bg-bg-primary"
                  >
                    <option value="dashboard">Dashboard</option>
                    <option value="services">Services</option>
                    <option value="logs">Logs</option>
                    <option value="fixtures">Fixtures</option>
                    <option value="metrics">Metrics</option>
                    <option value="testing">Testing</option>
                    <option value="config">Config</option>
                  </select>
                </div>
              </div>
            </TabsContent>
          </Tabs>
        </TabsProvider>

        {error && (
          <div className="text-sm text-destructive bg-destructive/10 p-3 rounded-md mt-4">
            {error}
          </div>
        )}

        <DialogFooter className="flex items-center justify-between">
          <Button
            type="button"
            variant="outline"
            onClick={handleReset}
            className="flex items-center gap-2"
          >
            <RotateCcw className="h-4 w-4" />
            Reset to Defaults
          </Button>

          <div className="flex items-center gap-3">
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={loading}
            >
              Cancel
            </Button>
            <Button
              type="button"
              onClick={handleSave}
              disabled={loading}
            >
              {loading ? 'Saving...' : 'Save Preferences'}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
