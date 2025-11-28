# CDN Setup for MockForge Cloud

This guide explains how to set up a Content Delivery Network (CDN) for serving static assets in MockForge Cloud, improving performance and reducing server load.

## Overview

MockForge Cloud supports CDN integration for serving static assets (JavaScript, CSS, images, fonts) to improve:
- **Performance**: Faster asset delivery from edge locations
- **Reliability**: Reduced server load and bandwidth costs
- **Global Reach**: Lower latency for users worldwide
- **Caching**: Better cache control and invalidation

## Supported CDN Providers

MockForge Cloud is compatible with any CDN that supports:
- Static file hosting
- Custom domain configuration
- Cache invalidation API
- HTTPS support

### Recommended Providers

1. **Cloudflare** (Recommended for simplicity)
   - Free tier available
   - Automatic HTTPS
   - Easy setup
   - Global edge network

2. **AWS CloudFront**
   - Integrates with S3
   - Advanced caching rules
   - Origin shield support
   - Pay-as-you-go pricing

3. **Fastly**
   - High performance
   - Real-time cache purging
   - Advanced edge computing
   - Enterprise-focused

4. **Bunny CDN**
   - Cost-effective
   - Simple setup
   - Good performance
   - Pay-as-you-go

## Configuration

### Environment Variables

Set the following environment variables to enable CDN:

```bash
# CDN Base URL (e.g., https://cdn.mockforge.dev or https://d1234567890.cloudfront.net)
CDN_BASE_URL=https://cdn.mockforge.dev

# Optional: CDN for specific asset types
CDN_ASSETS_URL=https://assets.mockforge.dev  # Override for assets only
CDN_IMAGES_URL=https://images.mockforge.dev  # Override for images only

# Optional: CDN cache invalidation
CDN_INVALIDATION_ENABLED=true
CDN_INVALIDATION_API_KEY=your-api-key  # Provider-specific
```

### Application Configuration

The CDN base URL is read from environment variables and used to:
1. Rewrite asset URLs in HTML responses
2. Set proper cache headers
3. Enable cache invalidation (if configured)

## Setup Instructions

### Option 1: Cloudflare (Recommended)

1. **Create a Cloudflare account** and add your domain
2. **Create a CNAME record** pointing to your CDN subdomain:
   ```
   cdn.mockforge.dev -> your-origin-server.com
   ```
3. **Configure Cloudflare**:
   - Enable "Always Use HTTPS"
   - Set caching level to "Standard"
   - Configure cache rules for static assets
4. **Set environment variable**:
   ```bash
   CDN_BASE_URL=https://cdn.mockforge.dev
   ```

### Option 2: AWS CloudFront

1. **Create an S3 bucket** for static assets:
   ```bash
   aws s3 mb s3://mockforge-assets
   ```
2. **Upload assets** to S3:
   ```bash
   aws s3 sync dist/ s3://mockforge-assets/ --delete
   ```
3. **Create CloudFront distribution**:
   - Origin: S3 bucket or custom origin (your API server)
   - Default root object: `index.html`
   - Enable HTTPS
   - Configure cache behaviors
4. **Set environment variable**:
   ```bash
   CDN_BASE_URL=https://d1234567890.cloudfront.net
   ```

### Option 3: Custom CDN

1. **Upload static assets** to your CDN provider
2. **Configure CDN** with your origin server
3. **Set environment variable**:
   ```bash
   CDN_BASE_URL=https://your-cdn-domain.com
   ```

## Asset Deployment

### Build and Deploy Assets

1. **Build the frontend**:
   ```bash
   cd crates/mockforge-ui/ui
   npm run build
   ```

2. **Upload to CDN** (example for S3):
   ```bash
   # Upload to S3
   aws s3 sync dist/ s3://mockforge-assets/ \
     --cache-control "public, max-age=31536000, immutable" \
     --exclude "*.html" \
     --exclude "index.html"

   # Upload HTML with shorter cache
   aws s3 sync dist/ s3://mockforge-assets/ \
     --cache-control "public, max-age=3600, must-revalidate" \
     --include "*.html"
   ```

3. **Invalidate CDN cache** (if needed):
   ```bash
   # CloudFront
   aws cloudfront create-invalidation \
     --distribution-id E1234567890 \
     --paths "/*"
   ```

## Cache Strategy

### Hashed Assets (Long Cache)

Assets with content hashes (e.g., `index.abc123.js`) can be cached indefinitely:
```
Cache-Control: public, max-age=31536000, immutable
```

### Unhashed Assets (Short Cache)

Assets without hashes (e.g., `index.html`) need shorter cache:
```
Cache-Control: public, max-age=3600, must-revalidate
```

### Cache Invalidation

When deploying new versions:
1. **Hashed assets**: No invalidation needed (new hash = new URL)
2. **Unhashed assets**: Invalidate cache via CDN API
3. **HTML files**: Always invalidate to ensure users get latest version

## Implementation Details

### URL Rewriting

The application automatically rewrites asset URLs when `CDN_BASE_URL` is set:

```html
<!-- Before -->
<script src="/assets/index.js"></script>

<!-- After (with CDN) -->
<script src="https://cdn.mockforge.dev/assets/index.js"></script>
```

### Fallback Behavior

If CDN is unavailable or misconfigured:
- Assets fall back to origin server
- Application continues to function normally
- Errors are logged for monitoring

### Development vs Production

- **Development**: CDN is disabled by default
- **Production**: CDN is enabled when `CDN_BASE_URL` is set

## Monitoring

### Key Metrics

Monitor the following to ensure CDN is working correctly:
- **Cache hit ratio**: Should be > 90%
- **Origin requests**: Should be minimal
- **Response times**: Should be < 100ms from edge
- **Error rates**: Should be < 0.1%

### Health Checks

The application includes CDN health checks:
- `/health/cdn`: Checks CDN connectivity
- Logs CDN-related errors for debugging

## Troubleshooting

### Assets Not Loading from CDN

1. **Check environment variable**:
   ```bash
   echo $CDN_BASE_URL
   ```

2. **Verify CDN configuration**:
   - Ensure CNAME records are correct
   - Check CDN provider dashboard
   - Verify SSL certificates

3. **Check application logs**:
   ```bash
   # Look for CDN-related errors
   grep -i cdn /var/log/mockforge/app.log
   ```

### Cache Not Updating

1. **Invalidate CDN cache**:
   ```bash
   # Use CDN provider's invalidation API
   ```

2. **Check cache headers**:
   ```bash
   curl -I https://cdn.mockforge.dev/assets/index.js
   ```

3. **Verify asset hashes**:
   - New builds should have new hashes
   - Old hashes should still work (for backward compatibility)

## Best Practices

1. **Use hashed filenames**: Enables long-term caching
2. **Separate CDN for images**: Use `CDN_IMAGES_URL` for image assets
3. **Monitor cache hit rates**: Optimize cache rules based on metrics
4. **Set up alerts**: Monitor CDN errors and performance
5. **Test fallback**: Ensure origin server can serve assets if CDN fails
6. **Version assets**: Use content hashing for cache busting

## Cost Optimization

### Bandwidth Costs

- **CDN**: Typically $0.01-0.10 per GB
- **Origin server**: Higher costs, especially for high traffic
- **Savings**: CDN can reduce origin bandwidth by 80-90%

### Cache Hit Ratio

Target cache hit ratio: **> 90%**
- Higher hit ratio = lower costs
- Optimize cache rules to achieve this

## Security

### HTTPS Only

Always use HTTPS for CDN:
- Prevents man-in-the-middle attacks
- Required for modern browsers
- Improves SEO

### Content Security Policy

Update CSP headers to allow CDN:
```
Content-Security-Policy: default-src 'self' https://cdn.mockforge.dev
```

### Subresource Integrity

Consider using SRI for critical assets:
```html
<script src="https://cdn.mockforge.dev/assets/index.js"
        integrity="sha384-..."
        crossorigin="anonymous"></script>
```

## Migration Guide

### From Origin to CDN

1. **Set up CDN** (follow provider instructions)
2. **Upload assets** to CDN
3. **Set `CDN_BASE_URL`** environment variable
4. **Deploy application** with CDN support
5. **Monitor** for 24-48 hours
6. **Gradually increase** cache TTL as confidence grows

### Rollback

If issues occur:
1. **Remove `CDN_BASE_URL`** environment variable
2. **Redeploy** application
3. **Assets will fall back** to origin server
4. **Investigate** CDN configuration issues

---

For additional support, see the [Support Page](/support) or [Documentation](https://docs.mockforge.dev).
