# CDN Configuration Guide

## Overview

This guide covers setting up a Content Delivery Network (CDN) for MockForge to improve performance and reduce latency for static assets.

## Supported CDN Providers

1. **AWS CloudFront** - Recommended for AWS deployments
2. **Cloudflare** - Best for ease of use and DDoS protection
3. **Fastly** - Best for advanced control and edge computing
4. **Self-hosted NGINX** - For complete control

## CloudFront Setup (AWS)

### Prerequisites
- AWS account
- AWS CLI configured
- Domain name with Route53

### Deployment

```bash
# Create CloudFront distribution
aws cloudfront create-distribution \
  --distribution-config file://k8s/cdn-config.yaml

# Get distribution domain name
DISTRIBUTION_DOMAIN=$(aws cloudfront list-distributions \
  --query "DistributionList.Items[?Comment=='MockForge CDN Distribution'].DomainName" \
  --output text)

# Create Route53 alias record
aws route53 change-resource-record-sets \
  --hosted-zone-id YOUR_ZONE_ID \
  --change-batch '{
    "Changes": [{
      "Action": "CREATE",
      "ResourceRecordSet": {
        "Name": "cdn.mockforge.example.com",
        "Type": "A",
        "AliasTarget": {
          "HostedZoneId": "Z2FDTNDATAQYW2",
          "DNSName": "'$DISTRIBUTION_DOMAIN'",
          "EvaluateTargetHealth": false
        }
      }
    }]
  }'
```

### Cache Behaviors

- `/static/*` - Cache for 1 year (immutable)
- `/api/*` - No cache (bypass)
- Other paths - Cache for 4 hours

## Cloudflare Setup

### Prerequisites
- Cloudflare account
- Domain nameservers pointed to Cloudflare

### Configuration

1. **Add domain to Cloudflare**
```bash
# Using Terraform
terraform apply -var="cloudflare_api_token=$CF_API_TOKEN"
```

2. **Configure Page Rules**

Navigate to Dashboard → Page Rules:

| Rule | Pattern | Settings |
|------|---------|----------|
| Static Assets | `mockforge.example.com/static/*` | Cache Level: Cache Everything, Edge TTL: 1 year |
| API Bypass | `mockforge.example.com/api/*` | Cache Level: Bypass |

3. **Enable Performance Features**
- ✅ Auto Minify (CSS, JS, HTML)
- ✅ Brotli compression
- ✅ HTTP/2 and HTTP/3
- ✅ Rocket Loader

## Fastly Setup

### Prerequisites
- Fastly account
- API token

### VCL Configuration

```bash
# Upload VCL configuration
fastly vcl upload --version=latest --name=mockforge \
  --main --file=k8s/cdn-config/fastly-config.vcl
```

### Edge Computing

Fastly supports edge computing for dynamic content:

```vcl
sub vcl_recv {
  # Edge logic example: A/B testing
  if (req.url ~ "^/api/users" && rand.random(0, 100) < 50) {
    set req.backend = F_beta_backend;
  }
}
```

## Self-Hosted NGINX CDN

### Deployment

```bash
# Deploy NGINX CDN pods
kubectl apply -f - <<EOF
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: nginx-cdn
  namespace: mockforge
spec:
  selector:
    matchLabels:
      app: nginx-cdn
  template:
    metadata:
      labels:
        app: nginx-cdn
    spec:
      containers:
      - name: nginx
        image: nginx:alpine
        volumeMounts:
        - name: config
          mountPath: /etc/nginx/conf.d
        - name: cache
          mountPath: /var/cache/nginx
      volumes:
      - name: config
        configMap:
          name: cdn-config
          items:
          - key: nginx-cdn.conf
            path: default.conf
      - name: cache
        emptyDir:
          sizeLimit: 10Gi
EOF
```

## Cache Headers

MockForge sets appropriate cache headers:

```rust
use axum::http::header::{CACHE_CONTROL, EXPIRES};

// Static assets
response.headers_mut().insert(
    CACHE_CONTROL,
    "public, max-age=31536000, immutable".parse().unwrap()
);

// API responses
response.headers_mut().insert(
    CACHE_CONTROL,
    "no-store, no-cache, must-revalidate".parse().unwrap()
);

// Dynamic content with short cache
response.headers_mut().insert(
    CACHE_CONTROL,
    "public, max-age=300, must-revalidate".parse().unwrap()
);
```

## Cache Purging

### CloudFront

```bash
# Invalidate all
aws cloudfront create-invalidation \
  --distribution-id DISTRIBUTION_ID \
  --paths "/*"

# Invalidate specific path
aws cloudfront create-invalidation \
  --distribution-id DISTRIBUTION_ID \
  --paths "/static/*"
```

### Cloudflare

```bash
# Purge everything
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer $CF_API_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"purge_everything":true}'

# Purge specific files
curl -X POST "https://api.cloudflare.com/client/v4/zones/ZONE_ID/purge_cache" \
  -H "Authorization: Bearer $CF_API_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"files":["https://mockforge.example.com/static/app.js"]}'
```

### Fastly

```bash
# Purge by key
curl -X POST "https://api.fastly.com/service/SERVICE_ID/purge/static-assets" \
  -H "Fastly-Key: $FASTLY_API_KEY"
```

## Monitoring

### Metrics to Track

- Cache hit ratio
- Origin requests
- Edge response time
- Bandwidth savings
- Cache storage usage

### CloudWatch (AWS)

```bash
# View CloudFront metrics
aws cloudwatch get-metric-statistics \
  --namespace AWS/CloudFront \
  --metric-name Requests \
  --dimensions Name=DistributionId,Value=DISTRIBUTION_ID \
  --start-time 2025-10-06T00:00:00Z \
  --end-time 2025-10-07T00:00:00Z \
  --period 3600 \
  --statistics Sum
```

## Performance Testing

```bash
# Test cache performance
curl -I https://cdn.mockforge.example.com/static/logo.png
# Look for: X-Cache: HIT

# Test different regions
for region in us-east-1 eu-west-1 ap-southeast-1; do
  echo "Testing from $region"
  aws ec2 run-instances --region $region --user-data "curl -o /dev/null -s -w 'Time: %{time_total}s\n' https://cdn.mockforge.example.com/static/app.js"
done
```

## Best Practices

1. **Version static assets** - Use content hashing in filenames
2. **Compress content** - Enable gzip/brotli compression
3. **Set long cache times** - Use 1 year for immutable assets
4. **Use CDN for all static content** - Images, CSS, JS
5. **Monitor cache hit ratio** - Target >90% hit rate
6. **Implement cache warming** - Pre-load popular content
7. **Use edge computing** - For personalization at edge

## Costs

Estimated monthly costs (1TB transfer):

- **CloudFront**: ~$85
- **Cloudflare**: $20 (Pro plan)
- **Fastly**: ~$120
- **Self-hosted**: $50-100 (compute + bandwidth)

## Troubleshooting

### Low Cache Hit Ratio

- Check if query strings are being ignored
- Verify cache key configuration
- Review cookie handling

### Stale Content

- Implement versioning for static assets
- Set up automated cache invalidation
- Use short TTLs for dynamic content

### High Origin Load

- Increase cache TTLs
- Implement origin shield
- Enable edge caching for more paths
