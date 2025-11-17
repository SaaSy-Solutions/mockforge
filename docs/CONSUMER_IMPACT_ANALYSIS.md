# Consumer Impact Analysis

## Overview

Consumer Impact Analysis helps teams understand which applications and services will be affected when API contracts change. By mapping endpoints to client SDK methods and consuming applications, MockForge provides visibility into the downstream impact of contract drift.

## Concepts

### Consumer Mapping

A consumer mapping links an API endpoint to:
- **SDK Methods**: Client library methods that call the endpoint
- **Consuming Applications**: Applications that use those SDK methods

### App Types

Consuming applications are categorized by type:
- **Web**: Web applications (React, Vue, Angular, etc.)
- **Mobile iOS**: iOS mobile applications
- **Mobile Android**: Android mobile applications
- **Internal Tool**: Internal tools and dashboards
- **CLI**: Command-line tools
- **Other**: Other application types

### Impact Analysis

When contract drift is detected, MockForge analyzes:
1. Which endpoints are affected
2. Which SDK methods use those endpoints
3. Which applications use those SDK methods
4. The severity of the impact based on the type of change

## Configuration

### Creating a Consuming Application

```yaml
# Example: Register a consuming application
POST /api/v1/drift/consumer-mappings
{
  "endpoint": "/api/v1/users",
  "method": "GET",
  "sdk_methods": [
    {
      "sdk_name": "user-sdk-js",
      "method_name": "getUser",
      "consuming_apps": [
        {
          "app_id": "web-app-frontend",
          "name": "Web App Frontend",
          "type": "web",
          "repo_url": "https://github.com/company/web-app",
          "description": "Main customer-facing web application"
        }
      ]
    }
  ]
}
```

### SDK Method Registration

SDK methods represent client library functions that call specific endpoints:

```json
{
  "sdk_name": "user-sdk-js",
  "method_name": "getUser",
  "consuming_apps": [
    {
      "app_id": "mobile-ios-app",
      "name": "Mobile iOS App",
      "type": "mobile_ios",
      "repo_url": "https://github.com/company/mobile-ios"
    },
    {
      "app_id": "mobile-android-app",
      "name": "Mobile Android App",
      "type": "mobile_android",
      "repo_url": "https://github.com/company/mobile-android"
    }
  ]
}
```

## Usage

### Viewing Consumer Impact

When a drift incident occurs, you can view the consumer impact:

```bash
GET /api/v1/drift/incidents/{incident_id}/impact
```

Response:
```json
{
  "endpoint": "/api/v1/users",
  "method": "GET",
  "affected_apps": [
    {
      "app_id": "web-app-frontend",
      "name": "Web App Frontend",
      "type": "web",
      "sdk_methods": [
        {
          "sdk_name": "user-sdk-js",
          "method_name": "getUser"
        }
      ]
    },
    {
      "app_id": "mobile-ios-app",
      "name": "Mobile iOS App",
      "type": "mobile_ios",
      "sdk_methods": [
        {
          "sdk_name": "user-sdk-ios",
          "method_name": "fetchUser"
        }
      ]
    }
  ]
}
```

### Listing Consumer Mappings

List all consumer mappings:

```bash
GET /api/v1/drift/consumer-mappings
```

Query parameters:
- `endpoint`: Filter by endpoint
- `method`: Filter by HTTP method

### Looking Up a Specific Mapping

Find the consumer mapping for a specific endpoint:

```bash
GET /api/v1/drift/consumer-mappings/lookup?endpoint=/api/v1/users&method=GET
```

## Admin UI

### Consumer Impact Panel

The Consumer Impact Panel appears in the Incident Dashboard when viewing drift incidents. It shows:

1. **Affected Applications**: List of applications that may be impacted
2. **SDK Methods**: Which SDK methods are affected
3. **Impact Summary**: Overview of the potential impact

### Visual Indicators

- **Web Apps**: 🌐 Globe icon
- **Mobile iOS**: 📱 iPhone icon
- **Mobile Android**: 🤖 Android icon
- **Internal Tools**: 🛠️ Tool icon
- **CLI**: 💻 Terminal icon

## Best Practices

### 1. Keep Mappings Up to Date

- Update consumer mappings when new applications are deployed
- Remove mappings for deprecated applications
- Update SDK method names when client libraries are refactored

### 2. Comprehensive Coverage

- Map all production applications
- Include staging environments for early detection
- Track internal tools and admin dashboards

### 3. Integration with CI/CD

- Automatically register new applications during deployment
- Update mappings as part of SDK releases
- Validate mappings in integration tests

### 4. Impact Assessment

- Review consumer impact before deploying breaking changes
- Notify affected teams proactively
- Coordinate rollouts with downstream teams

## Example Workflow

1. **Register Applications**: When deploying a new application, register it with the consumer mapping system
2. **Map SDK Methods**: Link SDK methods to the endpoints they call
3. **Monitor Drift**: When contract drift is detected, view the consumer impact
4. **Assess Impact**: Review which applications will be affected
5. **Coordinate Response**: Work with affected teams to plan updates or coordinate rollouts

## API Reference

### Create Consumer Mapping

```http
POST /api/v1/drift/consumer-mappings
Content-Type: application/json

{
  "endpoint": "/api/v1/users",
  "method": "GET",
  "sdk_methods": [...]
}
```

### List Consumer Mappings

```http
GET /api/v1/drift/consumer-mappings?endpoint=/api/v1/users&method=GET
```

### Get Incident Impact

```http
GET /api/v1/drift/incidents/{incident_id}/impact
```

## See Also

- [Drift Budgets](./DRIFT_BUDGETS.md) - Configure drift thresholds
- [Fitness Functions](./DRIFT_BUDGETS.md#fitness-functions) - Define contract quality rules
- [Incident Management](./DRIFT_BUDGETS.md#incidents) - Manage drift incidents
