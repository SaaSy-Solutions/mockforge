# Risk Register Template

**Purpose:** Template for documenting risks in the risk register
**Compliance:** SOC 2 CC3, ISO 27001 Clause 6

---

## Risk Register Entry

### Basic Information

- **Risk ID**: Unique identifier (e.g., RISK-001)
- **Title**: Brief risk description
- **Description**: Detailed risk description
- **Category**: Risk category (Technical, Operational, Compliance, Business)
- **Subcategory**: Specific risk area
- **Date Identified**: Date risk was identified
- **Identified By**: Person/team who identified risk
- **Status**: Current status (Open, In Progress, Closed, Accepted)

### Risk Analysis

- **Threat**: Threat description
- **Vulnerability**: Vulnerability description
- **Asset**: Affected asset(s)
- **Likelihood**: Probability (1-5)
- **Impact**: Severity (1-5)
- **Risk Score**: Likelihood × Impact (1-25)
- **Risk Level**: Critical, High, Medium, Low

### Current Controls

- **Existing Controls**: List of existing controls
- **Control Effectiveness**: Effectiveness rating
- **Control Gaps**: Identified gaps

### Risk Treatment

- **Treatment Option**: Avoid, Mitigate, Transfer, Accept
- **Treatment Plan**: Detailed treatment plan
- **Treatment Owner**: Person/team responsible
- **Treatment Deadline**: Target completion date
- **Treatment Status**: Not Started, In Progress, Completed, On Hold
- **Treatment Cost**: Estimated cost
- **Treatment Benefits**: Expected benefits

### Residual Risk

- **Residual Likelihood**: Likelihood after treatment (1-5)
- **Residual Impact**: Impact after treatment (1-5)
- **Residual Risk Score**: Residual likelihood × residual impact
- **Residual Risk Level**: Critical, High, Medium, Low
- **Acceptable**: Whether residual risk is acceptable

### Review

- **Last Reviewed**: Date of last review
- **Next Review**: Date of next review
- **Review Frequency**: How often risk is reviewed
- **Reviewer**: Person who reviewed
- **Review Notes**: Review findings and updates

### Compliance

- **SOC 2 Criteria**: Relevant SOC 2 criteria
- **ISO 27001 Controls**: Relevant ISO 27001 controls
- **Other Standards**: Other compliance requirements

---

## Example Risk Entry

```yaml
risk:
  # Basic Information
  id: "RISK-001"
  title: "Unauthorized Access to Admin UI"
  description: |
    Risk of unauthorized access to admin interface leading to
    system compromise, data breach, or configuration changes.
  category: "Technical"
  subcategory: "Access Control"
  date_identified: "2025-01-15"
  identified_by: "security-team"
  status: "In Progress"

  # Risk Analysis
  threat: "External attacker gains unauthorized access to admin UI"
  vulnerability: "Weak authentication, missing MFA, insufficient monitoring"
  asset: "Admin UI, system configuration, user data"
  likelihood: 3  # Possible
  impact: 4  # High
  risk_score: 12  # High
  risk_level: "High"

  # Current Controls
  existing_controls:
    - "Authentication required (JWT, OAuth2, API keys)"
    - "Role-based access control (RBAC)"
    - "Audit logging enabled"
    - "Network security (firewall, TLS)"
  control_effectiveness: "Partial"
  control_gaps:
    - "MFA not enforced for admin access"
    - "Insufficient monitoring and alerting"
    - "No automated access reviews"

  # Risk Treatment
  treatment_option: "Mitigate"
  treatment_plan:
    - "Implement MFA for all admin users (Q1 2025)"
    - "Enhance security monitoring and alerting (Q1 2025)"
    - "Implement automated access reviews (Q1 2025)"
    - "Conduct security awareness training (Q2 2025)"
  treatment_owner: "security-team"
  treatment_deadline: "2025-03-31"
  treatment_status: "In Progress"
  treatment_cost: "$15,000"
  treatment_benefits:
    - "Reduced risk of unauthorized access"
    - "Improved compliance with SOC 2 and ISO 27001"
    - "Enhanced security posture"

  # Residual Risk
  residual_likelihood: 2  # Unlikely
  residual_impact: 3  # Medium
  residual_risk_score: 6  # Medium
  residual_risk_level: "Medium"
  acceptable: true

  # Review
  last_reviewed: "2025-01-27"
  next_review: "2025-04-27"
  review_frequency: "Quarterly"
  reviewer: "security-team-lead"
  review_notes: |
    Treatment plan in progress. MFA implementation scheduled
    for Q1 2025. Monitoring enhancements in progress.

  # Compliance
  compliance_requirements:
    soc2:
      - "CC6 (Logical and Physical Access)"
      - "CC4 (Monitoring Activities)"
    iso27001:
      - "A.9.2 (User Access Management)"
      - "A.9.4 (System and Application Access Control)"
      - "A.12.4 (Logging and Monitoring)"
```

---

## Risk Categories

### Technical Risks
- Access control
- Authentication/authorization
- Data protection
- System security
- Network security
- Application security

### Operational Risks
- Process failures
- Human error
- Third-party risks
- Business continuity
- Incident response

### Compliance Risks
- Regulatory violations
- Audit findings
- Policy violations
- Certification failures

### Business Risks
- Reputation damage
- Financial impact
- Operational disruption
- Customer impact

---

## Risk Status Workflow

1. **Open**: Risk identified, not yet treated
2. **In Progress**: Treatment plan in execution
3. **On Hold**: Treatment temporarily paused
4. **Completed**: Treatment completed, residual risk acceptable
5. **Accepted**: Risk accepted with monitoring
6. **Closed**: Risk no longer applicable

---

## Risk Review Checklist

- [ ] Risk still relevant
- [ ] Likelihood assessment current
- [ ] Impact assessment current
- [ ] Risk score updated
- [ ] Treatment plan on track
- [ ] Residual risk acceptable
- [ ] Compliance requirements met
- [ ] Review date scheduled

---

**Last Updated:** 2025-01-27
