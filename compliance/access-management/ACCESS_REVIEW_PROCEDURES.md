# Access Review Procedures

**Purpose:** Automated access review procedures for compliance
**Compliance:** SOC 2 CC6 (Logical Access), ISO 27001 A.9.2 (User Access Management)

---

## Overview

Regular access reviews ensure that users have appropriate access rights and that access is revoked when no longer needed. This document defines automated access review procedures.

---

## Access Review Types

### 1. User Access Review

**Frequency:** Quarterly
**Scope:** All users and their assigned roles/permissions
**Process:**

1. **Generate Access Report**
   - List all active users
   - Current roles and permissions
   - Last login date
   - Access granted date
   - Resource access details

2. **Review Criteria**
   - User still employed/active
   - Role still appropriate
   - Permissions still needed
   - Last access within retention period

3. **Automated Actions**
   - Flag inactive users (>90 days)
   - Flag users with excessive permissions
   - Flag users with no recent access
   - Generate review report

4. **Review Workflow**
   - Send review request to manager
   - Manager reviews and approves/rejects
   - Auto-revoke if not approved within timeframe
   - Log all review actions

### 2. Privileged Access Review

**Frequency:** Monthly
**Scope:** Users with admin/privileged roles
**Process:**

1. **Identify Privileged Users**
   - Admin role users
   - Users with elevated permissions
   - Service accounts with high privileges

2. **Review Criteria**
   - Justification for privileged access
   - Recent privileged actions
   - Compliance with least privilege
   - Multi-factor authentication enabled

3. **Automated Actions**
   - Require MFA for privileged users
   - Flag users without MFA
   - Generate privileged access report
   - Alert on privilege escalation

### 3. API Token Review

**Frequency:** Monthly
**Scope:** All active API tokens
**Process:**

1. **Generate Token Report**
   - List all active tokens
   - Token owner
   - Token scopes/permissions
   - Last usage date
   - Creation date
   - Expiration date

2. **Review Criteria**
   - Token still needed
   - Scopes still appropriate
   - Recent usage
   - Expiration status

3. **Automated Actions**
   - Revoke unused tokens (>90 days)
   - Flag tokens with excessive scopes
   - Rotate tokens approaching expiration
   - Generate review report

### 4. Resource Access Review

**Frequency:** Quarterly
**Scope:** Access to sensitive resources
**Process:**

1. **Identify Sensitive Resources**
   - Resources with sensitive data
   - Resources with high permissions
   - Resources with compliance requirements

2. **Review Criteria**
   - Users with access
   - Access level appropriateness
   - Recent access
   - Business justification

3. **Automated Actions**
   - Generate resource access report
   - Flag excessive access
   - Alert on sensitive resource access
   - Log all access reviews

---

## Automated Review Configuration

### Configuration File

```yaml
# config.yaml
security:
  access_review:
    enabled: true

    # User access review
    user_review:
      enabled: true
      frequency: "quarterly"  # monthly, quarterly, annually
      inactive_threshold: "90d"  # Days of inactivity
      auto_revoke_inactive: true
      require_manager_approval: true
      approval_timeout: "30d"  # Days to approve or auto-revoke

    # Privileged access review
    privileged_review:
      enabled: true
      frequency: "monthly"
      require_mfa: true
      require_justification: true
      alert_on_escalation: true

    # API token review
    token_review:
      enabled: true
      frequency: "monthly"
      unused_threshold: "90d"  # Days of non-usage
      auto_revoke_unused: true
      rotation_threshold: "30d"  # Days before expiration

    # Resource access review
    resource_review:
      enabled: true
      frequency: "quarterly"
      sensitive_resources:
        - "billing"
        - "user_data"
        - "audit_logs"
        - "security_settings"

    # Notification settings
    notifications:
      enabled: true
      channels:
        - email
        - slack
      recipients:
        - security_team
        - compliance_team
        - managers
```

### Environment Variables

```bash
export MOCKFORGE_ACCESS_REVIEW_ENABLED=true
export MOCKFORGE_ACCESS_REVIEW_FREQUENCY=quarterly
export MOCKFORGE_ACCESS_REVIEW_INACTIVE_THRESHOLD=90d
export MOCKFORGE_ACCESS_REVIEW_AUTO_REVOKE=true
```

---

## Review Reports

### User Access Report

```json
{
  "review_id": "review-2025-Q1",
  "review_date": "2025-01-27",
  "review_type": "user_access",
  "total_users": 150,
  "users_reviewed": 150,
  "findings": {
    "inactive_users": 12,
    "excessive_permissions": 5,
    "no_recent_access": 8,
    "privileged_without_mfa": 2
  },
  "actions_taken": {
    "users_revoked": 3,
    "permissions_reduced": 5,
    "mfa_enforced": 2
  },
  "pending_reviews": 7,
  "next_review_date": "2025-04-27"
}
```

### Privileged Access Report

```json
{
  "review_id": "review-2025-01-privileged",
  "review_date": "2025-01-27",
  "review_type": "privileged_access",
  "total_privileged_users": 15,
  "users_reviewed": 15,
  "findings": {
    "without_mfa": 2,
    "without_justification": 1,
    "excessive_privileges": 3
  },
  "actions_taken": {
    "mfa_enforced": 2,
    "justification_required": 1,
    "privileges_reduced": 3
  },
  "next_review_date": "2025-02-27"
}
```

### API Token Report

```json
{
  "review_id": "review-2025-01-tokens",
  "review_date": "2025-01-27",
  "review_type": "api_token",
  "total_tokens": 45,
  "tokens_reviewed": 45,
  "findings": {
    "unused_tokens": 8,
    "expiring_soon": 5,
    "excessive_scopes": 3
  },
  "actions_taken": {
    "tokens_revoked": 8,
    "tokens_rotated": 5,
    "scopes_reduced": 3
  },
  "next_review_date": "2025-02-27"
}
```

---

## Review Workflow

### Automated Review Process

1. **Schedule Review**
   - System schedules review based on frequency
   - Generates review report
   - Identifies users/resources to review

2. **Send Review Requests**
   - Send review request to managers
   - Include access details
   - Set approval deadline

3. **Manager Review**
   - Manager reviews access
   - Approves or requests changes
   - Provides justification if needed

4. **Auto-Revocation**
   - If not approved within timeout
   - Automatically revoke access
   - Notify user and manager
   - Log revocation

5. **Report Generation**
   - Generate review report
   - Document all actions
   - Store for compliance audit

---

## Access Review API

### Get Review Status

```bash
GET /api/v1/security/access-reviews
Authorization: Bearer ${ADMIN_TOKEN}

Response:
{
  "reviews": [
    {
      "review_id": "review-2025-Q1",
      "review_type": "user_access",
      "status": "pending",
      "due_date": "2025-02-15",
      "users_count": 150,
      "pending_approvals": 7
    }
  ]
}
```

### Approve Access Review

```bash
POST /api/v1/security/access-reviews/{review_id}/approve
Authorization: Bearer ${MANAGER_TOKEN}
Content-Type: application/json

{
  "user_id": "user-123",
  "approved": true,
  "justification": "User still requires access for project work"
}

Response:
{
  "review_id": "review-2025-Q1",
  "user_id": "user-123",
  "status": "approved",
  "approved_by": "manager-456",
  "approved_at": "2025-01-27T10:30:00Z"
}
```

### Revoke Access

```bash
POST /api/v1/security/access-reviews/{review_id}/revoke
Authorization: Bearer ${MANAGER_TOKEN}
Content-Type: application/json

{
  "user_id": "user-123",
  "reason": "User no longer requires access"
}

Response:
{
  "review_id": "review-2025-Q1",
  "user_id": "user-123",
  "status": "revoked",
  "revoked_by": "manager-456",
  "revoked_at": "2025-01-27T10:30:00Z"
}
```

---

## Compliance Requirements

### SOC 2 CC6 (Logical Access)
- ✅ Regular access reviews
- ✅ Access revocation procedures
- ✅ Privileged access management
- ✅ Access review documentation

### ISO 27001 A.9.2 (User Access Management)
- ✅ User access review
- ✅ Access provisioning/de-provisioning
- ✅ Access review records
- ✅ Regular access reviews

---

## Best Practices

1. **Regular Reviews**: Conduct reviews on schedule
2. **Documentation**: Document all review actions
3. **Automation**: Automate review processes where possible
4. **Timely Action**: Take action on review findings promptly
5. **Audit Trail**: Maintain complete audit trail of reviews

---

**Last Updated:** 2025-01-27
