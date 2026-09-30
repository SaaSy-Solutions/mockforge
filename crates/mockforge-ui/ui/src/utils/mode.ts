// Re-export the single cloud-mode detector. This module used to key only off
// VITE_API_BASE_URL, so a bundle built with just VITE_MOCKFORGE_MODE=cloud
// sent these callers (e.g. pillar analytics) to self-hosted endpoints.
import { isCloudMode } from './cloudMode';

export { isCloudMode };

export const IS_CLOUD = isCloudMode();
