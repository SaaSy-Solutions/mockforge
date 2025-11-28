# Outstanding Items - Organization Management

## ✅ Completed

### Core Organization Endpoints
- ✅ **POST /api/v1/organizations** - Create organization
- ✅ **GET /api/v1/organizations** - List organizations
- ✅ **GET /api/v1/organizations/:org_id** - Get organization details
- ✅ **GET /api/v1/organizations/:org_id/members** - List organization members

### Infrastructure
- ✅ Database schema (organizations, org_members tables)
- ✅ Models (Organization, OrgMember, OrgRole)
- ✅ TIMESTAMP/TIMESTAMPTZ type fixes
- ✅ Authentication and authorization
- ✅ Validation (slug format, uniqueness)
- ✅ All endpoints tested and working

## ⏭️ Missing/Outstanding

### 1. Member Management Endpoints (High Priority)
**Status**: Backend models exist, but no API handlers

**Missing endpoints**:
- `POST /api/v1/organizations/:org_id/members` - Add member to organization
- `DELETE /api/v1/organizations/:org_id/members/:user_id` - Remove member
- `PATCH /api/v1/organizations/:org_id/members/:user_id` - Update member role (admin/member)
- `POST /api/v1/organizations/:org_id/invitations` - Invite user by email (optional)

**Backend support**:
- ✅ `OrgMember::create()` exists
- ✅ `OrgMember::delete()` exists
- ✅ `OrgMember::update_role()` exists
- ❌ No handlers implemented

### 2. Organization Update Endpoint (Medium Priority)
**Status**: Not implemented

**Missing endpoint**:
- `PATCH /api/v1/organizations/:org_id` - Update organization (name, slug, plan)
- `DELETE /api/v1/organizations/:org_id` - Delete organization (with proper cleanup)

**Backend support**:
- ✅ `Organization::update_plan()` exists
- ❌ No update handler for name/slug
- ❌ No delete handler

### 3. Organization Settings Endpoints (Low Priority)
**Status**: Not implemented

**Missing endpoints**:
- `GET /api/v1/organizations/:org_id/settings` - Get organization settings
- `PATCH /api/v1/organizations/:org_id/settings` - Update organization settings
- `GET /api/v1/organizations/:org_id/usage` - Get organization usage stats
- `GET /api/v1/organizations/:org_id/billing` - Get organization billing info

### 4. Code Cleanup (Low Priority)
**Status**: Minor warnings only

**Issues**:
- Unused imports: `ApiToken`, `PluginWithVersions`, `Subscription`
- Unused variables in handlers (likely for future features)

### 5. Testing (Medium Priority)
**Status**: Core endpoints tested, advanced features not tested

**Missing tests**:
- Member management workflows
- Organization updates
- Plan upgrades/downgrades
- Organization deletion
- Integration with SSO, billing, hosted mocks

## Priority Recommendations

### Immediate (Before Production)
1. **Member Management Endpoints** - Critical for Team plan functionality
   - Add member
   - Remove member
   - Update member role
   - Proper authorization (only owners/admins can manage)

### Short-term (Post-Launch)
2. **Organization Update/Delete** - Needed for user management
   - Update organization details
   - Delete organization (with proper cleanup)

### Long-term (Nice to Have)
3. **Organization Settings** - Enhanced management
   - Settings management
   - Usage statistics
   - Billing integration

## Summary

**Core functionality**: ✅ **Complete and tested**
**Advanced features**: ⏭️ **Backend models ready, handlers needed**

The foundation is solid - all the database models and core endpoints work. The main gap is member management endpoints, which are essential for Team plan functionality.
