import { logger } from '@/utils/logger';
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
      logger.error('Failed to save preferences',error);
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

  const fontSizeOptions: { value: 'small' | 'medium' | 'large'; label: string; sample: string }[] = [
    { value: 'small', label: 'Small', sample: '14px' },
    { value: 'medium', label: 'Medium', sample: '16px' },
    { value: 'large', label: 'Large', sample: '18px' },
  ];

  // Hex values mirror the HSL palettes in useThemeSync.ts so the swatches
  // preview the actual applied accent (no Tailwind named color guesswork).
  const accentColors = [
    { value: 'blue', label: 'Blue', swatch: '#3B82F6' },
    { value: 'green', label: 'Green', swatch: '#16A34A' },
    { value: 'purple', label: 'Purple', swatch: '#9333EA' },
    { value: 'orange', label: 'Orange', swatch: '#C2410C' },
    { value: 'red', label: 'Red', swatch: '#DC2626' },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-4xl max-h-[90vh] overflow-y-auto bg-card">
        <DialogHeader className="space-y-2">
          <DialogTitle className="flex items-center gap-2 text-xl font-semibold text-foreground">
            <Settings className="h-5 w-5" />
            Preferences
          </DialogTitle>
          <DialogDescription className="text-sm text-muted-foreground leading-relaxed">
            Customize your experience with MockForge
          </DialogDescription>
          <DialogClose onClick={() => onOpenChange(false)} />
        </DialogHeader>

        <TabsProvider value={activeTab} onValueChange={setActiveTab}>
          <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
            <TabsList className="grid w-full grid-cols-5 bg-muted">
              <TabsTrigger value="theme" className="flex items-center gap-2 text-foreground data-[state=active]:text-foreground data-[state=active]:bg-card">
                <Palette className="h-4 w-4" />
                <span className="hidden sm:inline">Theme</span>
              </TabsTrigger>
              <TabsTrigger value="logs" className="flex items-center gap-2 text-foreground data-[state=active]:text-foreground data-[state=active]:bg-card">
                <FileText className="h-4 w-4" />
                <span className="hidden sm:inline">Logs</span>
              </TabsTrigger>
              <TabsTrigger value="notifications" className="flex items-center gap-2 text-foreground data-[state=active]:text-foreground data-[state=active]:bg-card">
                <Bell className="h-4 w-4" />
                <span className="hidden sm:inline">Notifications</span>
              </TabsTrigger>
              <TabsTrigger value="search" className="flex items-center gap-2 text-foreground data-[state=active]:text-foreground data-[state=active]:bg-card">
                <Search className="h-4 w-4" />
                <span className="hidden sm:inline">Search</span>
              </TabsTrigger>
              <TabsTrigger value="ui" className="flex items-center gap-2 text-foreground data-[state=active]:text-foreground data-[state=active]:bg-card">
                <Settings className="h-4 w-4" />
                <span className="hidden sm:inline">UI</span>
              </TabsTrigger>
            </TabsList>

            <TabsContent value="theme" className="space-y-6">
              <div className="space-y-4">
                <div>
                  <label className="text-sm font-medium text-foreground mb-3 block">
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
                            ? 'border-orange-500 bg-orange-50 dark:bg-orange-900/20 text-orange-700 dark:text-orange-300'
                            : 'border-border hover:border-orange-300 dark:hover:border-orange-600 text-foreground'
                        }`}
                      >
                        <Icon className="h-4 w-4" />
                        <span className="text-sm">{label}</span>
                      </button>
                    ))}
                  </div>
                </div>

                <div>
                  <label className="text-sm font-medium text-foreground mb-3 block">
                    Font Size
                  </label>
                  <div className="grid grid-cols-3 gap-2">
                    {fontSizeOptions.map(({ value, label, sample }) => (
                      <button
                        key={value}
                        onClick={() => updateTheme({ fontSize: value })}
                        className={`flex flex-col items-center gap-1 p-3 rounded-lg border transition-all ${
                          preferences.theme.fontSize === value
                            ? 'border-orange-500 bg-orange-50 dark:bg-orange-900/20 text-orange-700 dark:text-orange-300'
                            : 'border-border hover:border-orange-300 dark:hover:border-orange-600 text-foreground'
                        }`}
                      >
                        <span className="text-sm">{label}</span>
                        <span className="text-xs text-muted-foreground">{sample}</span>
                      </button>
                    ))}
                  </div>
                </div>

                <div>
                  <label className="text-sm font-medium text-foreground mb-3 block">
                    Accent Color
                  </label>
                  <div className="flex gap-2">
                    {accentColors.map(({ value, label, swatch }) => (
                      <button
                        key={value}
                        onClick={() => updateTheme({ accentColor: value as string })}
                        className={`w-8 h-8 rounded-full border-2 transition-all ${
                          preferences.theme.accentColor === value
                            ? 'border-gray-900 dark:border-gray-100 scale-110 shadow-lg'
                            : 'border-border hover:scale-105'
                        }`}
                        style={{ backgroundColor: swatch }}
                        title={label}
                        aria-label={label}
                      />
                    ))}
                  </div>
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-foreground">
                      High Contrast
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Auto-scroll
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Pause on Error
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Show Timestamps
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Compact View
                    </label>
                    <p className="text-xs text-muted-foreground">
                      Use compact layout for log entries
                    </p>
                  </div>
                  <Switch
                    checked={preferences.logs.compactView}
                    onCheckedChange={(checked) => updateLogs({ compactView: checked })}
                  />
                </div>

                <div>
                  <label className="text-sm font-medium text-foreground mb-2 block">
                    Default Time Range (hours)
                  </label>
                  <Input
                    type="number"
                    min="1"
                    max="168"
                    value={preferences.logs.defaultTimeRange}
                    onChange={(e) => updateLogs({ defaultTimeRange: parseInt(e.target.value) || 24 })}
                    className="w-24 bg-card text-foreground placeholder:text-muted-foreground dark:placeholder:text-muted-foreground"
                  />
                </div>

                <div>
                  <label className="text-sm font-medium text-foreground mb-2 block">
                    Items Per Page
                  </label>
                  <Input
                    type="number"
                    min="10"
                    max="1000"
                    step="10"
                    value={preferences.logs.itemsPerPage}
                    onChange={(e) => updateLogs({ itemsPerPage: parseInt(e.target.value) || 100 })}
                    className="w-24 bg-card text-foreground placeholder:text-muted-foreground dark:placeholder:text-muted-foreground"
                  />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="notifications" className="space-y-6">
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-foreground">
                      Enable Sounds
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Show Toasts
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Notify on Errors
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Notify on Success
                    </label>
                    <p className="text-xs text-muted-foreground">
                      Show notifications for successful operations
                    </p>
                  </div>
                  <Switch
                    checked={preferences.notifications.notifyOnSuccess}
                    onCheckedChange={(checked) => updateNotifications({ notifyOnSuccess: checked })}
                  />
                </div>

                <div>
                  <label className="text-sm font-medium text-foreground mb-2 block">
                    Toast Duration (seconds)
                  </label>
                  <Input
                    type="number"
                    min="1"
                    max="30"
                    value={preferences.notifications.toastDuration}
                    onChange={(e) => updateNotifications({ toastDuration: parseInt(e.target.value) || 5 })}
                    className="w-24 bg-card text-foreground placeholder:text-muted-foreground dark:placeholder:text-muted-foreground"
                  />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="search" className="space-y-6">
              <div className="space-y-4">
                <div>
                  <label className="text-sm font-medium text-foreground mb-3 block">
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
                    <label className="text-sm font-medium text-foreground">
                      Case Sensitive
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Regex Enabled
                    </label>
                    <p className="text-xs text-muted-foreground">
                      Allow regular expressions in search
                    </p>
                  </div>
                  <Switch
                    checked={preferences.search.regexEnabled}
                    onCheckedChange={(checked) => updateSearch({ regexEnabled: checked })}
                  />
                </div>

                <div>
                  <label className="text-sm font-medium text-foreground mb-2 block">
                    Max History Items
                  </label>
                  <Input
                    type="number"
                    min="5"
                    max="50"
                    value={preferences.search.maxHistoryItems}
                    onChange={(e) => updateSearch({ maxHistoryItems: parseInt(e.target.value) || 10 })}
                    className="w-24 bg-card text-foreground placeholder:text-muted-foreground dark:placeholder:text-muted-foreground"
                  />
                </div>
              </div>
            </TabsContent>

            <TabsContent value="ui" className="space-y-6">
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-foreground">
                      Sidebar Collapsed
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Confirm Delete
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Auto-save
                    </label>
                    <p className="text-xs text-muted-foreground">
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
                    <label className="text-sm font-medium text-foreground">
                      Keyboard Shortcuts
                    </label>
                    <p className="text-xs text-muted-foreground">
                      Enable keyboard shortcuts
                    </p>
                  </div>
                  <Switch
                    checked={preferences.ui.keyboardShortcuts}
                    onCheckedChange={(checked) => updateUI({ keyboardShortcuts: checked })}
                  />
                </div>

                <div>
                  <label className="text-sm font-medium text-foreground mb-2 block">
                    Default Page
                  </label>
                  <select
                    value={preferences.ui.defaultPage}
                    onChange={(e) => updateUI({ defaultPage: e.target.value })}
                    className="w-full p-2 border border-border rounded-md bg-card text-foreground"
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
