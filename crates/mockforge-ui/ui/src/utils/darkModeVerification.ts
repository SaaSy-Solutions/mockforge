/**
 * Dark Mode Verification Utility
 * 
 * This utility helps verify that components are using theme variables
 * and dark mode classes correctly.
 */

export interface DarkModeVerificationResult {
  component: string;
  hasDarkMode: boolean;
  usesThemeVariables: boolean;
  usesHardcodedColors: boolean;
  issues: string[];
}

/**
 * Check if a component file uses dark mode properly
 */
export function verifyComponentDarkMode(
  componentName: string,
  fileContent: string
): DarkModeVerificationResult {
  const issues: string[] = [];
  let hasDarkMode = false;
  let usesThemeVariables = false;
  let usesHardcodedColors = false;

  // Check for dark: prefix (Tailwind dark mode)
  if (fileContent.includes('dark:')) {
    hasDarkMode = true;
  }

  // Check for theme variables (CSS custom properties)
  const themeVariablePatterns = [
    /bg-bg-primary|bg-bg-secondary|bg-bg-tertiary/,
    /text-text-primary|text-text-secondary|text-text-tertiary/,
    /border-border/,
    /bg-background|text-foreground/,
    /bg-card|text-card-foreground/,
    /bg-primary|text-primary/,
    /bg-secondary|text-secondary/,
    /bg-muted|text-muted/,
    /bg-accent|text-accent/,
  ];

  for (const pattern of themeVariablePatterns) {
    if (pattern.test(fileContent)) {
      usesThemeVariables = true;
      break;
    }
  }

  // Check for hardcoded colors (gray-*, white, black without dark:)
  const hardcodedColorPatterns = [
    /(?:^|[^dark:])text-gray-\d+/,
    /(?:^|[^dark:])bg-gray-\d+/,
    /(?:^|[^dark:])border-gray-\d+/,
    /text-white(?!.*dark:)/,
    /bg-white(?!.*dark:)/,
    /text-black(?!.*dark:)/,
    /bg-black(?!.*dark:)/,
  ];

  for (const pattern of hardcodedColorPatterns) {
    if (pattern.test(fileContent)) {
      usesHardcodedColors = true;
      issues.push(`Found hardcoded color that may not work in dark mode`);
      break;
    }
  }

  // Check if component has dark mode but no theme variables
  if (hasDarkMode && !usesThemeVariables) {
    issues.push('Uses dark: classes but may not use theme variables');
  }

  // Check if component has no dark mode support
  if (!hasDarkMode && !usesThemeVariables) {
    issues.push('No dark mode support detected');
  }

  return {
    component: componentName,
    hasDarkMode,
    usesThemeVariables,
    usesHardcodedColors,
    issues,
  };
}

/**
 * Get recommended theme variable replacements
 */
export const themeVariableReplacements: Record<string, string> = {
  'text-gray-900': 'text-text-primary dark:text-text-primary',
  'text-gray-700': 'text-text-secondary dark:text-text-secondary',
  'text-gray-600': 'text-text-tertiary dark:text-text-tertiary',
  'text-gray-500': 'text-text-tertiary dark:text-text-tertiary',
  'text-gray-400': 'text-text-tertiary dark:text-text-tertiary',
  'text-gray-300': 'text-text-tertiary dark:text-text-tertiary',
  'text-white': 'text-foreground dark:text-foreground',
  'bg-white': 'bg-background dark:bg-background',
  'bg-gray-50': 'bg-bg-secondary dark:bg-bg-secondary',
  'bg-gray-100': 'bg-bg-tertiary dark:bg-bg-tertiary',
  'bg-gray-800': 'bg-bg-primary dark:bg-bg-primary',
  'bg-gray-900': 'bg-bg-secondary dark:bg-bg-secondary',
  'border-gray-300': 'border-border dark:border-border',
  'border-gray-600': 'border-border dark:border-border',
  'border-gray-700': 'border-border dark:border-border',
  'border-gray-800': 'border-border dark:border-border',
};

/**
 * Get recommended replacement for a hardcoded color
 */
export function getThemeVariableReplacement(hardcodedColor: string): string | null {
  return themeVariableReplacements[hardcodedColor] || null;
}
