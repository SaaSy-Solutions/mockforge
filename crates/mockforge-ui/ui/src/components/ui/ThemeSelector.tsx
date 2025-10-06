import { logger } from '@/utils/logger';
import React from 'react';
import { Palette } from 'lucide-react';
import { predefinedThemes } from '../../themes';
import { useThemePaletteStore } from '../../stores/useThemePaletteStore';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from './select';

export function ThemeSelector() {
  const { selectedThemeId, setThemePalette } = useThemePaletteStore();

  const selectedTheme = predefinedThemes.find(theme => theme.id === selectedThemeId);

  return (
    <Select value={selectedThemeId} onValueChange={setThemePalette}>
      <SelectTrigger className="justify-start">
        <div className="flex items-center mr-2">
          <Palette className="mr-2 h-4 w-4" />
          <span className="truncate">
            {selectedTheme?.name || 'Select Theme'}
          </span>
        </div>
        <SelectValue placeholder="Select Theme" />
      </SelectTrigger>
      <SelectContent>
        {predefinedThemes.map((theme) => (
          <SelectItem key={theme.id} value={theme.id}>
            <div className="flex items-center gap-3">
              {/* Theme preview colors */}
              <div className="flex gap-1">
                <div
                  className="w-3 h-3 rounded-full border"
                  style={{ backgroundColor: theme.preview.primary }}
                />
                <div
                  className="w-3 h-3 rounded-full border"
                  style={{ backgroundColor: theme.preview.secondary }}
                />
                <div
                  className="w-3 h-3 rounded-full border"
                  style={{ backgroundColor: theme.preview.accent }}
                />
              </div>

              <div className="flex-1 text-left">
                <div className="font-medium text-sm">{theme.name}</div>
                <div className="text-xs text-muted-foreground truncate">
                  {theme.description}
                </div>
              </div>
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
