import React, { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/Card';
import { X, Cookie, Settings } from 'lucide-react';
import { logger } from '@/utils/logger';

const COOKIE_CONSENT_KEY = 'mockforge-cookie-consent';
const COOKIE_CONSENT_EXPIRY_DAYS = 365;

interface CookieConsent {
  accepted: boolean;
  timestamp: number;
  preferences?: {
    necessary: boolean;
    analytics: boolean;
    marketing: boolean;
  };
}

/**
 * Cookie Consent Banner Component
 *
 * Displays a GDPR-compliant cookie consent banner that allows users to:
 * - Accept all cookies
 * - Reject non-essential cookies
 * - Customize cookie preferences
 *
 * Only shows if user hasn't made a choice yet, or if consent has expired.
 */
export function CookieConsentBanner() {
  const [showBanner, setShowBanner] = useState(false);
  const [showPreferences, setShowPreferences] = useState(false);
  const [preferences, setPreferences] = useState({
    necessary: true, // Always true (required for site functionality)
    analytics: false,
    marketing: false,
  });

  useEffect(() => {
    // Check if user has already given consent
    const consent = getCookieConsent();

    if (!consent) {
      // No consent given yet - show banner
      setShowBanner(true);
    } else if (isConsentExpired(consent)) {
      // Consent expired - show banner again
      setShowBanner(true);
    } else {
      // Valid consent exists - apply preferences
      applyCookiePreferences(consent.preferences || { necessary: true, analytics: false, marketing: false });
    }
  }, []);

  const getCookieConsent = (): CookieConsent | null => {
    try {
      const stored = localStorage.getItem(COOKIE_CONSENT_KEY);
      if (!stored) return null;
      return JSON.parse(stored) as CookieConsent;
    } catch {
      return null;
    }
  };

  const isConsentExpired = (consent: CookieConsent): boolean => {
    const expiryTime = consent.timestamp + (COOKIE_CONSENT_EXPIRY_DAYS * 24 * 60 * 60 * 1000);
    return Date.now() > expiryTime;
  };

  const saveCookieConsent = (prefs: typeof preferences) => {
    const consent: CookieConsent = {
      accepted: true,
      timestamp: Date.now(),
      preferences: prefs,
    };
    localStorage.setItem(COOKIE_CONSENT_KEY, JSON.stringify(consent));
    applyCookiePreferences(prefs);
    setShowBanner(false);
    setShowPreferences(false);
  };

  const applyCookiePreferences = (prefs: typeof preferences) => {
    // Necessary cookies are always enabled (required for site functionality)
    // This includes authentication, session management, etc.

    // Analytics cookies (Sentry)
    if (prefs.analytics) {
      // Enable Sentry if not already enabled
      // Note: Sentry is initialized in main.tsx, but we can control its behavior
      // by setting a global flag that Sentry respects
      (window as any).__MOCKFORGE_ANALYTICS_ENABLED = true;
      logger.debug('Analytics cookies enabled');
    } else {
      // Disable Sentry tracking
      (window as any).__MOCKFORGE_ANALYTICS_ENABLED = false;
      // Note: Sentry is already initialized, but we can prevent future events
      // by setting this flag. For full compliance, Sentry should be initialized
      // conditionally in main.tsx based on consent.
      logger.debug('Analytics cookies disabled');
    }

    // Marketing cookies (not currently used, but prepared for future use)
    if (prefs.marketing) {
      (window as any).__MOCKFORGE_MARKETING_ENABLED = true;
      logger.debug('Marketing cookies enabled');
    } else {
      (window as any).__MOCKFORGE_MARKETING_ENABLED = false;
      logger.debug('Marketing cookies disabled');
    }
  };

  const handleAcceptAll = () => {
    const allAccepted = {
      necessary: true,
      analytics: true,
      marketing: true,
    };
    saveCookieConsent(allAccepted);
  };

  const handleRejectAll = () => {
    const onlyNecessary = {
      necessary: true,
      analytics: false,
      marketing: false,
    };
    saveCookieConsent(onlyNecessary);
  };

  const handleSavePreferences = () => {
    saveCookieConsent(preferences);
  };

  const handleCustomize = () => {
    setShowPreferences(true);
  };

  if (!showBanner) {
    return null;
  }

  return (
    <div className="fixed bottom-0 left-0 right-0 z-50 p-4 pointer-events-none">
      <Card className="max-w-4xl mx-auto shadow-lg pointer-events-auto border-2">
        <CardContent className="p-6">
          {!showPreferences ? (
            // Main banner view
            <div className="flex flex-col md:flex-row items-start md:items-center gap-4">
              <div className="flex-shrink-0">
                <Cookie className="h-8 w-8 text-primary" />
              </div>
              <div className="flex-1">
                <h3 className="text-lg font-semibold mb-2">We use cookies</h3>
                <p className="text-sm text-muted-foreground mb-4">
                  We use cookies to enhance your browsing experience, analyze site traffic, and personalize content.
                  By clicking "Accept All", you consent to our use of cookies. You can also customize your preferences
                  or learn more in our{' '}
                  <a
                    href="/privacy"
                    className="text-primary hover:underline"
                    onClick={(e) => {
                      e.preventDefault();
                      // Navigate to privacy page
                      window.location.hash = '#privacy';
                    }}
                  >
                    Privacy Policy
                  </a>
                  .
                </p>
                <div className="flex flex-wrap gap-3">
                  <Button
                    onClick={handleAcceptAll}
                    size="sm"
                    className="flex items-center gap-2"
                  >
                    Accept All
                  </Button>
                  <Button
                    onClick={handleRejectAll}
                    variant="outline"
                    size="sm"
                  >
                    Reject All
                  </Button>
                  <Button
                    onClick={handleCustomize}
                    variant="ghost"
                    size="sm"
                    className="flex items-center gap-2"
                  >
                    <Settings className="h-4 w-4" />
                    Customize
                  </Button>
                </div>
              </div>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setShowBanner(false)}
                className="flex-shrink-0"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          ) : (
            // Preferences view
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <h3 className="text-lg font-semibold">Cookie Preferences</h3>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => setShowPreferences(false)}
                >
                  <X className="h-4 w-4" />
                </Button>
              </div>

              <div className="space-y-4">
                {/* Necessary Cookies */}
                <div className="flex items-start justify-between p-4 border rounded-lg">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <h4 className="font-semibold">Necessary Cookies</h4>
                      <span className="text-xs bg-primary/10 text-primary px-2 py-0.5 rounded">Required</span>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      These cookies are essential for the website to function properly. They include authentication,
                      session management, and security features. These cookies cannot be disabled.
                    </p>
                  </div>
                  <div className="ml-4">
                    <input
                      type="checkbox"
                      checked={preferences.necessary}
                      disabled
                      className="w-5 h-5"
                    />
                  </div>
                </div>

                {/* Analytics Cookies */}
                <div className="flex items-start justify-between p-4 border rounded-lg">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <h4 className="font-semibold">Analytics Cookies</h4>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      These cookies help us understand how visitors interact with our website by collecting and
                      reporting information anonymously. This includes error tracking (Sentry) and performance monitoring.
                    </p>
                  </div>
                  <div className="ml-4">
                    <input
                      type="checkbox"
                      checked={preferences.analytics}
                      onChange={(e) => setPreferences({ ...preferences, analytics: e.target.checked })}
                      className="w-5 h-5"
                    />
                  </div>
                </div>

                {/* Marketing Cookies */}
                <div className="flex items-start justify-between p-4 border rounded-lg">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <h4 className="font-semibold">Marketing Cookies</h4>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      These cookies are used to deliver personalized advertisements and track campaign performance.
                      Currently not in use, but available for future features.
                    </p>
                  </div>
                  <div className="ml-4">
                    <input
                      type="checkbox"
                      checked={preferences.marketing}
                      onChange={(e) => setPreferences({ ...preferences, marketing: e.target.checked })}
                      className="w-5 h-5"
                    />
                  </div>
                </div>
              </div>

              <div className="flex gap-3 pt-4 border-t">
                <Button onClick={handleSavePreferences} className="flex-1">
                  Save Preferences
                </Button>
                <Button
                  onClick={() => setShowPreferences(false)}
                  variant="outline"
                >
                  Cancel
                </Button>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
