# Static Asset Optimization Guide

This document outlines the optimization strategies implemented for static assets in MockForge Cloud.

## Overview

Static assets (images, fonts, CSS, JavaScript) are optimized to reduce load times, bandwidth usage, and improve user experience.

## Implemented Optimizations

### 1. Build-Time Optimizations

#### Vite Configuration
- **Minification**: Terser minification with console.log removal in production
- **Code Splitting**: Automatic code splitting for vendor libraries
- **Asset Inlining**: Assets smaller than 4KB are inlined as base64
- **CSS Code Splitting**: CSS split into separate files for better caching
- **Bundle Analysis**: Rollup visualizer for bundle size analysis

#### Image Optimization
- **Static Copy Plugin**: Copies and optimizes images during build
- **Format Optimization**: Support for WebP format (smaller file sizes)
- **Responsive Images**: Utilities for generating srcset and sizes attributes

### 2. Runtime Optimizations

#### Caching Headers
- **Hashed Assets**: Long-term caching (1 year) with `immutable` directive
- **Unhashed Assets**: Shorter cache (1 hour) with `must-revalidate`
- **Cache-Control**: Proper cache headers for all static assets

#### Compression
- **Accept-Encoding**: Vary header for compression negotiation
- **Gzip/Brotli**: Server-level compression (handled by reverse proxy/CDN)

#### Resource Hints
- **Preload**: Critical assets (index.css, index.js) preloaded
- **Preconnect**: Font providers preconnected for faster font loading
- **Font Loading**: Async font loading with fallback

### 3. Image Optimization Strategies

#### Lazy Loading
- **Intersection Observer**: Images load when entering viewport
- **50px Margin**: Start loading 50px before image is visible
- **Native Lazy Loading**: Falls back to native `loading="lazy"` attribute

#### Responsive Images
- **Srcset Generation**: Multiple image sizes for different viewports
- **Sizes Attribute**: Proper sizing hints for browser
- **WebP Support**: Automatic WebP format when supported

#### Format Detection
- **WebP Detection**: Checks browser support for WebP
- **Format Fallback**: Falls back to PNG/JPG if WebP not supported
- **SVG Optimization**: SVG files kept as-is (already optimized)

### 4. Font Optimization

#### Google Fonts
- **Preconnect**: DNS lookup and TCP connection established early
- **Async Loading**: Fonts loaded asynchronously to prevent render blocking
- **Display Swap**: `font-display=swap` for faster text rendering
- **Subset Loading**: Only required font weights loaded

### 5. JavaScript/CSS Optimization

#### Code Splitting
- **Vendor Chunks**: Separate chunks for React, UI libraries, Chart.js
- **Route-Based Splitting**: Lazy loading for route components
- **Dynamic Imports**: Heavy components loaded on demand

#### Minification
- **Terser**: JavaScript minification with dead code elimination
- **CSS Minification**: CSS minified and purged of unused styles
- **Console Removal**: Console.log statements removed in production

## Cache Strategy

### Hashed Assets (Long Cache)
```
Cache-Control: public, max-age=31536000, immutable
```
- Assets with content hashes in filename
- Can be cached forever (browser checks hash for updates)
- Examples: `index.a1b2c3d4.js`, `vendor.e5f6g7h8.css`

### Unhashed Assets (Short Cache)
```
Cache-Control: public, max-age=3600, must-revalidate
```
- Assets without hashes
- Shorter cache with revalidation
- Examples: `favicon.ico`, `robots.txt`

## Image Formats

### Recommended Formats
1. **WebP**: Best compression, modern browsers
2. **PNG**: Transparency support, fallback
3. **SVG**: Vector graphics, logos, icons
4. **JPG**: Photos, complex images

### Format Selection
- Use WebP for photos and complex images
- Use PNG for images requiring transparency
- Use SVG for logos, icons, simple graphics
- Use JPG for photos without transparency needs

## Performance Metrics

### Target Metrics
- **First Contentful Paint (FCP)**: < 1.8s
- **Largest Contentful Paint (LCP)**: < 2.5s
- **Time to Interactive (TTI)**: < 3.8s
- **Total Bundle Size**: < 350KB gzipped
- **Image Load Time**: < 1s for above-the-fold images

### Monitoring
- Use browser DevTools Network tab
- Lighthouse performance audits
- Web Vitals monitoring (via Sentry or similar)

## Best Practices

### Image Usage
1. **Lazy Load**: Use `loading="lazy"` for below-the-fold images
2. **Responsive**: Use `srcset` and `sizes` for responsive images
3. **Format**: Prefer WebP with PNG/JPG fallback
4. **Dimensions**: Specify width/height to prevent layout shift
5. **Alt Text**: Always include descriptive alt text

### Asset Loading
1. **Preload Critical**: Preload critical CSS and JS
2. **Defer Non-Critical**: Defer non-critical scripts
3. **Async Fonts**: Load fonts asynchronously
4. **Resource Hints**: Use preconnect for external resources

### Build Process
1. **Analyze Bundle**: Run `npm run build:analyze` regularly
2. **Monitor Size**: Track bundle size over time
3. **Optimize Imports**: Avoid importing entire libraries
4. **Tree Shaking**: Ensure unused code is eliminated

## CDN Integration (Future)

When setting up a CDN:
1. **Edge Caching**: Cache static assets at edge locations
2. **Compression**: Enable Brotli compression
3. **Image Optimization**: Use CDN image transformation
4. **HTTP/2 Push**: Push critical assets
5. **Cache Purging**: Implement cache invalidation strategy

## Tools and Scripts

### Build Analysis
```bash
npm run build:analyze
```
Opens bundle visualization in browser

### Image Optimization
```bash
# Install image optimization tools
npm install -D sharp-cli

# Optimize images
npx sharp-cli --input public/*.png --output public/optimized/
```

### Performance Testing
```bash
# Lighthouse CI
npm install -g @lhci/cli
lhci autorun

# WebPageTest
# Use online tool: https://www.webpagetest.org/
```

## Troubleshooting

### Large Bundle Size
1. Check bundle analysis report
2. Identify large dependencies
3. Consider code splitting or lazy loading
4. Remove unused dependencies

### Slow Image Loading
1. Check image file sizes
2. Verify lazy loading is working
3. Consider WebP conversion
4. Check CDN/network latency

### Cache Issues
1. Verify cache headers in DevTools
2. Check asset hashing in filenames
3. Clear browser cache for testing
4. Verify CDN cache settings

---

For more information, see:
- [Vite Performance Guide](https://vitejs.dev/guide/performance.html)
- [Web.dev Image Optimization](https://web.dev/fast/#optimize-your-images)
- [MDN Resource Hints](https://developer.mozilla.org/en-US/docs/Web/Performance/dns-prefetch)
