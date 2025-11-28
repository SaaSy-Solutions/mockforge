# MOD API Review Process

**Pillars:** [DevX][Contracts]

**Version:** 1.0.0
**Last Updated:** 2025-01-27

A mock-first API review process that ensures API designs are validated before implementation.

## Overview

MOD API Review is a process that uses mocks to review API designs before implementation. It enables:

- **Early feedback** — Review API design before coding
- **Visual validation** — See mock responses, not just schemas
- **Contract validation** — Ensure implementation matches contract
- **Faster reviews** — Review mocks, not code

## Review Workflow

### Step 1: Contract Definition

**Who:** API Designer / Backend Developer

**Action:** Define API contract (OpenAPI, gRPC, etc.)

**Deliverable:** Contract file (e.g., `contracts/api-v2.yaml`)

**Example:**

```yaml
# contracts/api-v2.yaml
openapi: 3.0.0
info:
  title: Users API v2
  version: 2.0.0
paths:
  /api/v2/users/{id}:
    get:
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: User details
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
```

### Step 2: Mock Generation

**Who:** API Designer / Backend Developer

**Action:** Generate mock from contract

**Command:**

```bash
mockforge generate --from-openapi contracts/api-v2.yaml --output mocks/
mockforge serve --config mockforge.yaml --admin
```

**Deliverable:** Running mock server with Admin UI

### Step 3: Mock Review

**Who:** Review Team (Frontend, Backend, Product, QA)

**Action:** Review mock responses in Admin UI

**Checklist:**
- [ ] Response structure makes sense
- [ ] Field names are clear
- [ ] Data types are appropriate
- [ ] Error responses are defined
- [ ] Edge cases are covered

**Deliverable:** Review feedback / approval

### Step 4: Implementation

**Who:** Backend Developer

**Action:** Implement API to match contract

**Deliverable:** Backend implementation

### Step 5: Contract Validation

**Who:** CI/CD / Automated Tool

**Action:** Validate implementation against contract

**Command:**

```bash
mockforge validate \
  --contract contracts/api-v2.yaml \
  --target http://localhost:8080
```

**Deliverable:** Validation report

### Step 6: Mock Comparison

**Who:** Review Team

**Action:** Compare mock vs. implementation

**Command:**

```bash
mockforge mod review \
  --contract contracts/api-v2.yaml \
  --mock http://localhost:3000 \
  --implementation http://localhost:8080
```

**Deliverable:** Comparison report

### Step 7: Approval

**Who:** Review Team

**Action:** Approve if contract matches

**Deliverable:** Approved PR / Merge

## Review Checklist

### Contract Review

- [ ] Contract syntax is valid (OpenAPI/gRPC)
- [ ] Version is specified
- [ ] Breaking changes are documented
- [ ] Deprecations are marked
- [ ] Schemas are complete

### Mock Review

- [ ] Mock responses match contract
- [ ] Response data is realistic
- [ ] Error scenarios are covered
- [ ] Edge cases are handled
- [ ] Performance is acceptable

### Implementation Review

- [ ] Implementation matches contract
- [ ] Response schemas are correct
- [ ] Status codes are correct
- [ ] Error handling is appropriate
- [ ] Performance meets requirements

## Review Tools

### MockForge Admin UI

**Use for:** Visual mock review

**Features:**
- Browse endpoints
- View mock responses
- Test endpoints
- Compare responses

**Access:** `http://localhost:3000/__mockforge`

### Contract Validation

**Use for:** Automated validation

**Command:**

```bash
mockforge validate --contract contracts/api.yaml --target http://localhost:8080
```

**Output:**
- Validation report
- Contract violations
- Recommendations

### Mock Comparison

**Use for:** Comparing mock vs. implementation

**Command:**

```bash
mockforge mod review \
  --contract contracts/api.yaml \
  --mock http://localhost:3000 \
  --implementation http://localhost:8080
```

**Output:**
- Comparison report
- Differences
- Recommendations

## PR Review Process

### 1. PR Creation

**Include in PR:**
- Contract changes (if any)
- Implementation code
- Mock updates (if any)
- Test updates

**PR Description Template:**

```markdown
## API Changes

### Contract Changes
- [Link to contract file]
- Breaking changes: Yes/No
- Version: v2.0.0

### Implementation
- [Link to implementation]
- Matches contract: Yes/No

### Mock Updates
- [Link to mock files]
- Updated: Yes/No

### Testing
- [Link to tests]
- Coverage: X%
```

### 2. Automated Validation

**CI/CD runs:**
- Contract validation
- Mock comparison
- Integration tests

**Fail if:**
- Contract violations
- Mock mismatch
- Test failures

### 3. Manual Review

**Reviewers check:**
- Contract changes
- Mock responses
- Implementation quality
- Test coverage

### 4. Approval

**Approve if:**
- Contract is valid
- Mock matches contract
- Implementation matches contract
- Tests pass

## Review Best Practices

### 1. Review Contracts First

✅ **Do:**
- Review contract before implementation
- Validate contract syntax
- Check for breaking changes

❌ **Don't:**
- Review implementation first
- Skip contract review
- Ignore breaking changes

### 2. Use Mocks for Design Discussion

✅ **Do:**
- Show mock responses in review
- Discuss design with mocks
- Iterate on mock responses

❌ **Don't:**
- Discuss design without mocks
- Review only schemas
- Skip mock review

### 3. Validate Automatically

✅ **Do:**
- Validate in CI/CD
- Fail on contract violations
- Run validation on every PR

❌ **Don't:**
- Validate only manually
- Skip validation in CI
- Allow contract drift

### 4. Compare Mock vs. Implementation

✅ **Do:**
- Compare mock and implementation
- Check for differences
- Fix discrepancies

❌ **Don't:**
- Assume they match
- Skip comparison
- Ignore differences

## Common Review Scenarios

### Scenario 1: New API Endpoint

**Process:**
1. Define contract for new endpoint
2. Generate mock
3. Review mock responses
4. Implement endpoint
5. Validate implementation
6. Compare mock vs. implementation
7. Approve

### Scenario 2: API Version Update

**Process:**
1. Create new contract version
2. Generate mock for new version
3. Review breaking changes
4. Implement new version
5. Validate both versions
6. Compare versions
7. Approve

### Scenario 3: Contract Fix

**Process:**
1. Identify contract issue
2. Fix contract
3. Regenerate mock
4. Review mock changes
5. Update implementation (if needed)
6. Validate
7. Approve

## Troubleshooting

### Problem: Contract validation fails

**Solution:**
- Check contract syntax
- Validate against OpenAPI/gRPC spec
- Fix contract errors

### Problem: Mock doesn't match implementation

**Solution:**
- Update implementation to match contract
- Or update contract if design changed
- Regenerate mock

### Problem: Review takes too long

**Solution:**
- Use automated validation
- Review mocks, not code
- Focus on contract changes

## Further Reading

- [MOD Philosophy](MOD_PHILOSOPHY.md) — Core MOD principles
- [MOD Guide](MOD_GUIDE.md) — Step-by-step workflow
- [MOD Patterns](MOD_PATTERNS.md) — Common patterns

---

**MOD API Review makes API reviews faster, clearer, and more effective. Start using mocks in your review process today.**
