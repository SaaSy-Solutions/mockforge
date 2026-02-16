import React, { createContext, useContext, useMemo, useState } from 'react';
import { translations } from './translations';
import type { Locale } from './translations';

type I18nContextValue = {
  locale: Locale;
  supportedLocales: readonly Locale[];
  setLocale: (locale: Locale) => void;
  t: (key: string, fallback?: string) => string;
};

const I18nContext = createContext<I18nContextValue | undefined>(undefined);

const SUPPORTED_LOCALES: readonly Locale[] =
  import.meta.env.VITE_ENABLE_BETA_LOCALES === 'true' ? ['en', 'es'] : ['en'];

function resolveInitialLocale(): Locale {
  const saved = localStorage.getItem('mockforge-locale');
  if (saved === 'en' || saved === 'es') {
    if (SUPPORTED_LOCALES.includes(saved)) {
      return saved;
    }
  }

  if (SUPPORTED_LOCALES.includes('es')) {
    const browser = navigator.language.toLowerCase();
    if (browser.startsWith('es')) {
      return 'es';
    }
  }

  return 'en';
}

function normalizeLocale(locale: Locale): Locale {
  if (SUPPORTED_LOCALES.includes(locale)) {
    return locale;
  }
  return 'en';
}

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => resolveInitialLocale());

  const setLocale = (next: Locale) => {
    const normalized = normalizeLocale(next);
    localStorage.setItem('mockforge-locale', normalized);
    setLocaleState(normalized);
  };

  const value = useMemo<I18nContextValue>(
    () => ({
      locale,
      supportedLocales: SUPPORTED_LOCALES,
      setLocale,
      t: (key: string, fallback?: string) =>
        translations[locale][key] ?? translations.en[key] ?? fallback ?? key,
    }),
    [locale]
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const ctx = useContext(I18nContext);
  if (!ctx) {
    throw new Error('useI18n must be used inside I18nProvider');
  }
  return ctx;
}
