/**
 * Image optimization utilities
 *
 * Provides helpers for lazy loading, responsive images, and format optimization
 */

/**
 * Generate responsive image srcset for different sizes
 */
export function generateSrcSet(
  basePath: string,
  sizes: number[],
  format: 'png' | 'jpg' | 'webp' = 'png'
): string {
  return sizes
    .map(size => `${basePath.replace(/\.(png|jpg|jpeg|webp)$/, `-${size}w.${format}`)} ${size}w`)
    .join(', ');
}

/**
 * Generate sizes attribute for responsive images
 */
export function generateSizes(breakpoints: { max?: number; min?: number; size: string }[]): string {
  return breakpoints
    .map(bp => {
      if (bp.max && bp.min) {
        return `(min-width: ${bp.min}px) and (max-width: ${bp.max}px) ${bp.size}`;
      } else if (bp.min) {
        return `(min-width: ${bp.min}px) ${bp.size}`;
      } else if (bp.max) {
        return `(max-width: ${bp.max}px) ${bp.size}`;
      }
      return bp.size;
    })
    .join(', ');
}

/**
 * Check if WebP is supported by the browser
 */
export function supportsWebP(): boolean {
  if (typeof window === 'undefined') return false;

  const canvas = document.createElement('canvas');
  canvas.width = 1;
  canvas.height = 1;
  return canvas.toDataURL('image/webp').indexOf('data:image/webp') === 0;
}

/**
 * Get optimal image format based on browser support
 */
export function getOptimalImageFormat(basePath: string): string {
  if (supportsWebP() && !basePath.endsWith('.svg')) {
    return basePath.replace(/\.(png|jpg|jpeg)$/, '.webp');
  }
  return basePath;
}

/**
 * Lazy load image with intersection observer
 */
export function lazyLoadImage(
  img: HTMLImageElement,
  src: string,
  options?: IntersectionObserverInit
): () => void {
  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          img.src = src;
          img.loading = 'eager'; // Switch to eager once loaded
          observer.unobserve(img);
        }
      });
    },
    {
      rootMargin: '50px', // Start loading 50px before image enters viewport
      ...options,
    }
  );

  observer.observe(img);

  // Return cleanup function
  return () => observer.disconnect();
}

/**
 * Preload critical images
 */
export function preloadImage(src: string, as: 'image' | 'fetch' = 'image'): void {
  if (typeof document === 'undefined') return;

  const link = document.createElement('link');
  link.rel = 'preload';
  link.as = as;
  link.href = src;
  if (as === 'image') {
    link.setAttribute('fetchpriority', 'high');
  }
  document.head.appendChild(link);
}
