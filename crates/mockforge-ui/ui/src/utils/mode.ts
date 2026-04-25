export const isCloudMode = (): boolean => !!import.meta.env.VITE_API_BASE_URL;

export const IS_CLOUD = isCloudMode();
