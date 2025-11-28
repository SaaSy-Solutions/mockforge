# SIEM Integration Guide

**Purpose:** Integrate MockForge security events with Security Information and Event Management (SIEM) systems
**Compliance:** SOC 2 CC4 (Monitoring Activities), ISO 27001 A.12.4 (Logging and Monitoring)

---

## Overview

MockForge generates security events that can be integrated with SIEM systems for centralized security monitoring, threat detection, and compliance reporting.

---

## Security Event Types

### Authentication Events
- `auth.success` - Successful authentication
- `auth.failure` - Failed authentication attempt
- `auth.token_expired` - Token expiration
- `auth.token_revoked` - Token revocation
- `auth.mfa_enabled` - Multi-factor authentication enabled
- `auth.mfa_disabled` - Multi-factor authentication disabled
- `auth.password_changed` - Password change
- `auth.password_reset` - Password reset

### Authorization Events
- `authz.access_granted` - Access granted
- `authz.access_denied` - Access denied
- `authz.privilege_escalation` - Privilege escalation
- `authz.role_changed` - Role change
- `authz.permission_changed` - Permission change

### Access Management Events
- `access.user_created` - User account created
- `access.user_deleted` - User account deleted
- `access.user_suspended` - User account suspended
- `access.user_activated` - User account activated
- `access.api_token_created` - API token created
- `access.api_token_deleted` - API token deleted
- `access.api_token_rotated` - API token rotated

### Configuration Events
- `config.changed` - Configuration changed
- `config.security_policy_updated` - Security policy updated
- `config.encryption_key_rotated` - Encryption key rotated
- `config.tls_certificate_updated` - TLS certificate updated

### Data Events
- `data.exported` - Data export
- `data.deleted` - Data deletion
- `data.encrypted` - Data encryption
- `data.decrypted` - Data decryption
- `data.classified` - Data classification

### Security Events
- `security.vulnerability_detected` - Vulnerability detected
- `security.threat_detected` - Threat detected
- `security.anomaly_detected` - Anomaly detected
- `security.rate_limit_exceeded` - Rate limit exceeded
- `security.suspicious_activity` - Suspicious activity detected

### Compliance Events
- `compliance.audit_log_accessed` - Audit log accessed
- `compliance.compliance_check` - Compliance check performed
- `compliance.policy_violation` - Policy violation detected

---

## Event Format

### JSON Structure

```json
{
  "timestamp": "2025-01-27T10:30:00Z",
  "event_type": "auth.failure",
  "severity": "medium",
  "source": {
    "system": "mockforge",
    "component": "auth",
    "version": "1.0.0"
  },
  "actor": {
    "user_id": "user-123",
    "username": "admin",
    "ip_address": "192.168.1.100",
    "user_agent": "Mozilla/5.0..."
  },
  "target": {
    "resource_type": "api",
    "resource_id": "/api/v1/workspaces",
    "method": "GET"
  },
  "outcome": {
    "success": false,
    "reason": "Invalid credentials"
  },
  "metadata": {
    "auth_method": "jwt",
    "attempt_count": 3,
    "lockout_triggered": false
  },
  "compliance": {
    "soc2_cc": ["CC6"],
    "iso27001": ["A.9.2"]
  }
}
```

---

## SIEM Integration Methods

### 1. Syslog (RFC 5424)

**Configuration:**
```yaml
# config.yaml
security:
  monitoring:
    siem:
      enabled: true
      protocol: "syslog"
      format: "rfc5424"
      destinations:
        - host: "siem.example.com"
          port: 514
          protocol: "udp"
          facility: "local0"
          tag: "mockforge"
        - host: "siem-backup.example.com"
          port: 514
          protocol: "tcp"
          facility: "local0"
          tag: "mockforge"
```

**Environment Variables:**
```bash
export MOCKFORGE_SIEM_ENABLED=true
export MOCKFORGE_SIEM_PROTOCOL=syslog
export MOCKFORGE_SIEM_HOST=siem.example.com
export MOCKFORGE_SIEM_PORT=514
export MOCKFORGE_SIEM_FACILITY=local0
```

### 2. HTTP/HTTPS Webhook

**Configuration:**
```yaml
security:
  monitoring:
    siem:
      enabled: true
      protocol: "https"
      destinations:
        - url: "https://siem.example.com/api/v1/events"
          method: "POST"
          headers:
            Authorization: "Bearer ${SIEM_API_KEY}"
            X-SIEM-Source: "mockforge"
          timeout: 5
          retry:
            max_attempts: 3
            backoff: "exponential"
```

### 3. File-Based Export

**Configuration:**
```yaml
security:
  monitoring:
    siem:
      enabled: true
      protocol: "file"
      destinations:
        - path: "/var/log/mockforge/siem-events.jsonl"
          format: "jsonl"
          rotation:
            max_size: "100MB"
            max_files: 10
            compress: true
```

### 4. Cloud SIEM Integration

#### Splunk
```yaml
security:
  monitoring:
    siem:
      enabled: true
      protocol: "splunk"
      destinations:
        - url: "https://splunk.example.com:8088/services/collector"
          token: "${SPLUNK_TOKEN}"
          index: "mockforge_security"
          source_type: "mockforge:security"
```

#### Datadog
```yaml
security:
  monitoring:
    siem:
      enabled: true
      protocol: "datadog"
      destinations:
        - api_key: "${DATADOG_API_KEY}"
          app_key: "${DATADOG_APP_KEY}"
          site: "datadoghq.com"
          tags:
            - "service:mockforge"
            - "env:production"
```

#### AWS CloudWatch
```yaml
security:
  monitoring:
    siem:
      enabled: true
      protocol: "cloudwatch"
      destinations:
        - region: "us-east-1"
          log_group: "/aws/mockforge/security"
          stream: "events"
          credentials:
            access_key_id: "${AWS_ACCESS_KEY_ID}"
            secret_access_key: "${AWS_SECRET_ACCESS_KEY}"
```

#### Google Cloud Logging
```yaml
security:
  monitoring:
    siem:
      enabled: true
      protocol: "gcp"
      destinations:
        - project_id: "mockforge-prod"
          log_name: "security-events"
          credentials_path: "/path/to/service-account.json"
```

#### Azure Monitor
```yaml
security:
  monitoring:
    siem:
      enabled: true
      protocol: "azure"
      destinations:
        - workspace_id: "${AZURE_WORKSPACE_ID}"
          shared_key: "${AZURE_SHARED_KEY}"
          log_type: "MockForgeSecurity"
```

---

## Event Filtering

Filter events before sending to SIEM:

```yaml
security:
  monitoring:
    siem:
      filters:
        # Include only specific event types
        include:
          - "auth.*"
          - "authz.*"
          - "security.*"

        # Exclude low-severity events
        exclude:
          - "severity:low"

        # Include only events from production
        conditions:
          - "environment == 'production'"
```

---

## Alerting Rules

Define alerting rules for SIEM:

```yaml
security:
  monitoring:
    alerts:
      - name: "Multiple Auth Failures"
        condition: "event_type == 'auth.failure' AND count > 5 in 5m"
        severity: "high"
        action: "notify_security_team"

      - name: "Privilege Escalation"
        condition: "event_type == 'authz.privilege_escalation'"
        severity: "critical"
        action: "block_user"

      - name: "Suspicious Activity"
        condition: "event_type == 'security.suspicious_activity'"
        severity: "high"
        action: "investigate"
```

---

## Compliance Mapping

### SOC 2 Common Criteria
- **CC4 (Monitoring Activities)**: All security events
- **CC6 (Logical Access)**: Authentication and authorization events
- **CC7 (System Operations)**: Configuration and system events

### ISO 27001 Controls
- **A.9.2 (User Access Management)**: Access management events
- **A.9.4 (System Access Control)**: Authentication and authorization events
- **A.12.4 (Logging and Monitoring)**: All security events
- **A.16.1 (Incident Management)**: Security and threat events

---

## Testing

### Test Event Generation

```bash
# Generate test authentication failure event
curl -X POST http://localhost:9080/api/v1/security/test-event \
  -H "Authorization: Bearer ${ADMIN_TOKEN}" \
  -d '{
    "event_type": "auth.failure",
    "severity": "medium",
    "test": true
  }'
```

### Verify SIEM Integration

1. Check SIEM logs for received events
2. Verify event format matches SIEM requirements
3. Test alerting rules
4. Verify compliance reporting

---

## Best Practices

1. **Encrypt in Transit**: Always use TLS for SIEM connections
2. **Rate Limiting**: Implement rate limiting to prevent SIEM overload
3. **Retry Logic**: Implement exponential backoff for failed deliveries
4. **Event Retention**: Maintain local event logs for backup
5. **Monitoring**: Monitor SIEM integration health
6. **Testing**: Regularly test SIEM integration

---

## Troubleshooting

### Events Not Reaching SIEM

1. Check network connectivity
2. Verify SIEM credentials
3. Check event filters
4. Review application logs
5. Test with manual event generation

### Performance Issues

1. Implement event batching
2. Use async event delivery
3. Optimize event filtering
4. Consider event sampling for high-volume events

---

**Last Updated:** 2025-01-27
