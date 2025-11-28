# MockForge AI Studio

**Pillars:** [AI]

[AI] - LLM/voice flows, AI diff/assist, generative behaviors

## Overview

MockForge AI Studio is a unified interface for all AI-powered features in MockForge. It provides a cohesive "Mock AI Copilot" experience that unifies multiple AI capabilities under one productized surface. AI Studio serves as the main entry point for all AI features, while maintaining deep links to specialized tools like MockAI, Voice Interface, and Contract Diff.

**Key Principles:**
- **Unified UX**: One place to access all AI features
- **Context-Aware**: AI understands your workspace, contracts, personas, and reality settings
- **Deterministic Options**: Freeze AI outputs for reproducible testing
- **Enterprise Controls**: Org-level budgets, rate limits, and feature toggles

## Features

### 1. Natural Language Mock Generation (MockAI)

Generate complete OpenAPI specifications from natural language descriptions. Simply describe your API, and AI Studio will create a fully functional mock specification.

**Example:**
```
"Create a user API with CRUD operations for managing users"
```

### 2. AI-Guided Debugging

Analyze test failures and get AI-powered suggestions with comprehensive context from all MockForge subsystems. The enhanced debug analyzer:

- **Context-Aware Analysis**: Pulls context from:
  - **Reality Engine**: Current reality level, chaos config, latency profile
  - **Contract Validation**: Contract validation status, mismatches, drift
  - **Scenario Engine**: Active scenario, state, triggered rules
  - **Persona Registry**: Active persona, traits, relationships
  - **Chaos Engine**: Active chaos rules, fault injection, network profiles

- **Intelligent Suggestions**:
  - Explains which mock scenario/persona/reality setting likely caused the failure
  - Suggests specific fixes: "Tighten validation here" or "Add an explicit error example for this case"
  - Links to relevant artifacts for quick navigation

- **Automatic Fixes**: Provides JSON Patch operations for configuration corrections

**Example:**
```
Paste test failure logs → Get root cause analysis with context →
See which persona/reality setting caused it → Apply suggested fixes
```

**API Endpoint:**
```
POST /api/v1/ai-studio/debug/analyze-with-context
```

### 3. Persona Generation

Create realistic user personas with traits, backstories, and lifecycle states from natural language descriptions.

**Example:**
```
"Create a premium customer persona with high spending, active subscription, and priority support"
```

### 4. Contract Diff Analysis

Compare captured requests against OpenAPI specifications to identify mismatches and get AI-powered recommendations for corrections.

**Natural Language Queries:**
You can now ask questions about contract diffs directly in AI Studio chat:
- "Analyze the last captured request"
- "Show me breaking changes"
- "Compare contract versions"
- "Summarize drift for mobile endpoints"

The Contract Diff handler processes these queries and provides:
- Analysis results with mismatches and recommendations
- Breaking changes identification
- Links to the full Contract Diff Viewer page for detailed exploration

**API Endpoint:**
```
POST /api/v1/ai-studio/contract-diff/query
```

### 5. Usage Tracking & Budget Management

Track AI usage by feature (MockAI, Contract Diff, Persona Generation, Debug Analysis) with:
- Token usage per feature
- Cost estimates (USD)
- Call counts
- Budget limits and enforcement

### 6. Organization-Level AI Controls

Enterprise-grade controls for managing AI usage across organizations and workspaces:

**Budget Controls:**
- Maximum tokens per period (day/week/month/year)
- Maximum AI calls per period
- Per-organization and per-workspace limits

**Rate Limiting:**
- Requests per minute
- Optional per-hour and per-day limits
- Prevents API abuse and cost overruns

**Feature Toggles:**
- Enable/disable specific AI features per organization or workspace
- Examples: Enable Contract Diff, disable free-form generation
- Granular control over AI capabilities

**Usage Audit Logs:**
- Track all AI usage with timestamps
- User-level attribution
- Feature-level breakdown
- Cost tracking

**Configuration Model:**
- **YAML Defaults**: Baseline configuration in config files (for on-prem/self-hosted)
- **Database Authoritative**: Live settings stored in database (for cloud multi-tenancy)
- **Precedence**: Database overrides YAML if both exist

**API Endpoints:**
```
GET  /api/v1/ai-studio/org-controls
PUT  /api/v1/ai-studio/org-controls
GET  /api/v1/ai-studio/org-controls/usage
```

## AI Modes

### Live Mode (Default)

In **Live** mode, AI is used dynamically at runtime for each request. This provides maximum flexibility and real-time adaptation but incurs LLM costs for each operation.

**Use cases:**
- Development and experimentation
- Dynamic persona generation
- Real-time mock generation

### Generate Once Freeze Mode

In **Generate Once Freeze** mode, AI is only used to produce config/templates initially. At runtime, the system uses frozen artifacts (YAML/JSON files) with no LLM calls.

**Benefits:**
- **Deterministic behavior**: Same inputs produce same outputs
- **Zero runtime costs**: No LLM calls during test execution
- **Version control friendly**: Frozen artifacts can be committed to git
- **Reproducible tests**: Tests run identically across environments

**Freeze Modes:**

1. **Auto-Freeze**: Automatically freeze artifacts immediately after generation
   - Best for: CI/CD pipelines, production environments
   - Ensures all AI outputs are deterministic

2. **Manual Freeze**: Require explicit user action to freeze artifacts
   - Best for: Development, experimentation
   - Allows iterative refinement before freezing

**Metadata Tracking:**
Frozen artifacts include comprehensive metadata for reproducibility:
- LLM provider and model used
- LLM version (if available)
- Input prompt hash (for verification)
- Output hash (for integrity checking)
- Timestamp of freezing
- Original prompt/description

**How it works:**
1. Generate artifacts using AI (mocks, personas, scenarios)
2. (If auto-freeze enabled) Artifacts are automatically frozen
3. (If manual freeze) User clicks "Freeze this artifact"
4. Artifacts saved to `.mockforge/frozen/` directory with metadata
5. At runtime, system checks for frozen artifacts before making LLM calls
6. If frozen artifact exists, it's loaded instead of generating new content

**Use cases:**
- CI/CD pipelines (auto-freeze)
- Production testing (auto-freeze)
- Cost-sensitive environments (auto-freeze)
- Reproducible test suites (auto-freeze)
- Development and prototyping (manual freeze)

**API Endpoints:**
```
POST /api/v1/ai-studio/freeze
GET  /api/v1/ai-studio/frozen
```

## Unified UX & Navigation

AI Studio serves as the main entry point for all AI features, with breadcrumb navigation and quick links to specialized tools:

**Main Entry Point:**
- `/ai-studio` - Unified AI Copilot interface

**Specialized Tools (Deep Links):**
- `/mockai` - MockAI generation page
- `/voice` - Voice + LLM Interface
- `/contract-diff` - Contract Diff Viewer

**Navigation Features:**
- Breadcrumb navigation showing current location
- "Back to AI Studio" buttons on specialized pages
- Quick Actions panel with links to all AI tools
- Seamless integration between chat and specialized views

## Configuration

### Workspace-Level Settings

Configure AI mode per workspace in the **Config** page:

1. Navigate to **Config** → **General Settings**
2. Find **AI Mode** section
3. Select:
   - **Live**: AI used dynamically at runtime
   - **Generate Once Freeze**: Use frozen artifacts only

4. Configure **Deterministic Mode**:
   - **Mode**: Auto or Manual freeze
   - **Track Metadata**: Enable/disable metadata tracking
   - **Freeze Format**: YAML or JSON
   - **Freeze Directory**: Where to store frozen artifacts

### Organization-Level Settings

Organization admins can configure:
- Maximum AI calls per workspace per day/month
- Feature flags (enable/disable specific AI features)
- Budget limits

**API Endpoints:**
- `GET /api/v1/organizations/:org_id/settings/ai` - Get AI settings
- `PATCH /api/v1/organizations/:org_id/settings/ai` - Update AI settings

## Freezing Artifacts

### Freezing a Mock

After generating a mock in AI Studio:

1. Use the **Freeze** button or API endpoint
2. Artifact is saved to `.mockforge/frozen/` directory
3. Metadata is added indicating:
   - `frozen_at`: Timestamp
   - `artifact_type`: Type (mock, persona, scenario)
   - `source`: "ai_generated"

### Freezing a Persona

1. Generate persona in **Personas** tab
2. Use freeze API or UI action
3. Persona is saved with metadata

### Loading Frozen Artifacts

In **Generate Once Freeze** mode:
- System automatically checks for frozen artifacts before LLM calls
- Artifacts are matched by description hash
- Latest matching artifact is loaded

## UI Indicators

### AI-Generated Badge

Scenarios and personas created with AI show an **AI** badge with sparkles icon (✨).

### Frozen Badge

Artifacts that are frozen (deterministic mode) show a **Frozen** badge with snowflake icon (❄️).

These indicators help you identify:
- Which artifacts were AI-generated
- Which artifacts are using frozen deterministic content
- The source and mode of each artifact

## Usage Dashboard

The **Budget** tab in AI Studio provides:

### Overall Statistics
- Total tokens used
- Total cost (USD)
- Total calls made
- Budget usage percentage

### Feature Breakdown
Per-feature usage tracking:
- **MockAI**: Natural language mock generation
- **Contract Diff**: Contract analysis and recommendations
- **Persona Generation**: AI-generated personas
- **Debug Analysis**: AI-guided debugging
- **Generative Schema**: Schema generation from examples
- **Voice Interface**: Voice commands and chat
- **General Chat**: General assistant interactions

Each feature shows:
- Tokens used
- Cost (USD)
- Number of calls
- Percentage of total usage

## Governance

### Budget Limits

Organization-level budget controls:
- `max_ai_calls_per_workspace_per_day`: Daily call limit
- `max_ai_calls_per_workspace_per_month`: Monthly call limit

### Feature Flags

Control which AI features are enabled:
- `ai_studio_enabled`: Enable/disable AI Studio
- `ai_contract_diff_enabled`: Enable/disable Contract Diff
- `mockai_enabled`: Enable/disable MockAI
- `persona_generation_enabled`: Enable/disable Persona Generation
- `generative_schema_enabled`: Enable/disable Generative Schema
- `voice_interface_enabled`: Enable/disable Voice Interface

### Cost Estimation

Costs are estimated based on:
- **OpenAI GPT-3.5**: ~$0.002 per 1K tokens
- **OpenAI GPT-4**: ~$0.03 per 1K tokens
- **Anthropic Claude**: ~$0.008 per 1K tokens
- **Ollama**: $0 (local models)

## Best Practices

### For Development
- Use **Live** mode for rapid iteration
- Experiment with different prompts
- Monitor usage in Budget tab

### For CI/CD
- Use **Generate Once Freeze** mode
- Freeze all AI-generated artifacts
- Commit frozen artifacts to version control
- Ensure deterministic test execution

### For Production
- Use **Generate Once Freeze** mode
- Set appropriate budget limits
- Monitor usage dashboard regularly
- Enable only necessary feature flags

## API Reference

### Chat Endpoint
```
POST /api/v1/ai-studio/chat
```

Process natural language commands and get AI responses.

### Generate Mock
```
POST /api/v1/ai-studio/generate-mock
```

Generate OpenAPI specification from description.

### Generate Persona
```
POST /api/v1/ai-studio/generate-persona
```

Generate persona from description.

### Debug Test
```
POST /api/v1/ai-studio/debug-test
```

Analyze test failure and get suggestions.

### Freeze Artifact
```
POST /api/v1/ai-studio/freeze
```

Freeze an AI-generated artifact to deterministic format.

### Get Usage
```
GET /api/v1/ai-studio/usage?workspace_id=<id>
```

Get usage statistics for a workspace.

### Apply Patch
```
POST /api/v1/ai-studio/apply-patch
```

Apply a JSON Patch suggestion to configuration.

## Troubleshooting

### Frozen Artifacts Not Loading

1. Check that `ai_mode` is set to `generate_once_freeze`
2. Verify frozen artifacts exist in `.mockforge/frozen/`
3. Ensure artifact type and identifier match

### High Costs

1. Review feature breakdown in Budget tab
2. Consider switching to **Generate Once Freeze** mode
3. Set organization-level budget limits
4. Disable unused feature flags

### Feature Not Working

1. Check organization-level feature flags
2. Verify workspace has access to AI features
3. Check budget limits haven't been exceeded

## Future Enhancements

- Enhanced artifact matching (semantic similarity)
- Automatic artifact freezing on generation
- Artifact versioning and rollback
- Multi-workspace usage aggregation
- Cost alerts and notifications
