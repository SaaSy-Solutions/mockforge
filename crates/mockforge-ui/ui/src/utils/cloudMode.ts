// Single source of truth for "is this bundle talking to the SaaS registry
// (cloud mode) or to the embedded admin server (local / self-hosted mode)".
//
// Cloud mode is opt-in via VITE_MOCKFORGE_MODE=cloud. That keeps OSS and
// docker-compose builds in local mode by default, which is what the embedded
// /__mockforge/auth/* endpoints expect. Historically detection keyed off
// VITE_API_BASE_URL being set; we still honor that as a fallback so existing
// cloud deploys keep working, but new cloud builds should set the explicit
// flag.

export const isCloudMode = (): boolean => {
  const mode = import.meta.env.VITE_MOCKFORGE_MODE;
  if (typeof mode === 'string' && mode.toLowerCase() === 'cloud') {
    return true;
  }
  // Legacy fallback: treat a non-empty VITE_API_BASE_URL as cloud mode.
  const apiBase = import.meta.env.VITE_API_BASE_URL;
  return typeof apiBase === 'string' && apiBase.trim() !== '';
};

export const getCloudApiBase = (): string => {
  const apiBase = import.meta.env.VITE_API_BASE_URL;
  return typeof apiBase === 'string' ? apiBase : '';
};
