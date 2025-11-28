# Persona Integration Verification Guide

This document outlines how to verify that persona-based response generation is working correctly.

## Runtime Verification Steps

### 1. Verify Persona Loading from Config

**Test:** Start MockForge and check logs for persona loading messages.

**Expected Output:**
```
INFO mockforge_http: Loaded active persona 'commercial_midwest' from config file: tools/mockforge/config.yaml
INFO mockforge_http: Using persona 'commercial_midwest' for route generation
```

**How to Test:**
1. Ensure `tools/mockforge/config.yaml` has personas configured under `mockai.intelligent_behavior.personas`
2. Start MockForge server
3. Check startup logs for persona loading messages

### 2. Verify Persona Passed Through Route Generation

**Test:** Check that routes have persona attached.

**Expected Behavior:**
- Routes generated from OpenAPI spec should have `persona` field set
- Persona name should match the active persona from config

**How to Test:**
1. Start MockForge with OpenAPI spec
2. Check route registry - routes should have persona attached
3. Verify persona name matches config

### 3. Verify Persona Traits Used During Response Generation

**Test:** Generate a response that should use persona traits for count inference.

**Expected Behavior:**
- When generating `/api/apiaries/{apiaryId}/hives` response
- If no explicit `total` in response schema
- System should check persona traits for `hive_count`
- Should generate array with count based on persona trait (e.g., 75 for "50-100" range)

**How to Test:**
1. Configure persona with `hive_count: "50-100"` trait
2. Make GET request to `/api/apiaries/apiary_001/hives`
3. Check response - should have `items` array
4. If persona trait is used, array should have approximately 75 items (midpoint of 50-100)
5. Check logs for: "Using persona trait 'hive_count' for pagination: total=75"

### 4. Verify Count Inference from Persona Traits

**Test:** Verify that persona numeric traits are correctly parsed and used.

**Expected Behavior:**
- Range values like "50-100" should be parsed to midpoint (75)
- Single values like "50" should be used as-is
- Trait should be used when no explicit total is found in schema

**How to Test:**
1. Set persona trait: `hive_count: "50-100"`
2. Generate response for endpoint without explicit total
3. Verify response uses 75 (midpoint) as total
4. Change trait to `hive_count: "100"` and verify it uses 100

## Manual Test Script

```bash
# 1. Start MockForge with config
cd /home/rclanan/dev/projects/work/apiary-pro-saas
docker-compose -f tools/docker/docker-compose.local.yml up -d

# 2. Check logs for persona loading
docker-compose -f tools/docker/docker-compose.local.yml logs mockforge | grep -i persona

# 3. Make request to hives endpoint
curl http://localhost:3002/api/apiaries/apiary_001/hives | jq '.items | length'

# 4. Verify count matches persona trait
# If persona has hive_count: "50-100", should see ~75 items
```

## Automated Test Status

Unit tests for persona functionality:
- ✅ `test_persona_get_numeric_trait` - Verifies trait parsing
- ✅ `test_personas_config_get_active_persona` - Verifies persona selection
- ⚠️ Integration tests - Blocked by unrelated compilation errors in test suite

## Known Issues

1. **Config Structure:** Config file must have personas under `mockai.intelligent_behavior.personas` (not top-level `intelligent_behavior`)
2. **Config File Location:** Loader checks multiple paths, but may not find file if not in expected location
3. **Test Compilation:** Some unrelated compilation errors prevent full test suite from running

## Next Steps

1. Fix unrelated compilation errors in test suite
2. Run full integration test suite
3. Verify end-to-end with actual MockForge server
4. Add logging to verify persona usage in production
