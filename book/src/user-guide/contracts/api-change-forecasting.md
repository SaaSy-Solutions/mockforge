# API Change Forecasting

**Pillars:** [Contracts]

API Change Forecasting uses historical sync/diff data to predict likely future contract breaks. This lets teams proactively harden clients before changes occur.

## Overview

Instead of reacting to breaking changes, API Change Forecasting analyzes historical patterns to predict:
- **When** changes are likely to occur
- **What type** of changes (breaking vs non-breaking)
- **Which endpoints** are most volatile
- **Seasonal patterns** in changes

This is hugely enterprise-friendly—it transforms contract management from reactive to proactive.

## Key Features

- **Pattern Analysis**: Detects seasonal patterns, volatility, and change frequency
- **Statistical Modeling**: Predicts change probability and break probability
- **Multi-Window Forecasting**: 30, 90, and 180-day forecasts
- **Hierarchical Aggregation**: Workspace, service, and endpoint-level predictions
- **Confidence Scoring**: Indicates how reliable each forecast is

## How It Works

### 1. Historical Analysis

The system analyzes historical drift incidents to identify patterns:

- **Change Frequency**: How often does this endpoint change?
- **Change Types**: What types of changes occur (breaking vs non-breaking)?
- **Seasonal Patterns**: Are changes more common at certain times?
- **Volatility Score**: How stable is this endpoint?

### 2. Statistical Modeling

Using the historical data, the system builds statistical models:

- **Change Probability**: Likelihood of any change in the forecast window
- **Break Probability**: Likelihood of a breaking change
- **Next Change Date**: Expected date of next change (if predictable)
- **Confidence**: How reliable is this forecast?

### 3. Forecasting

Forecasts are generated for multiple time windows:

- **30-day forecast**: Short-term predictions
- **90-day forecast**: Medium-term predictions
- **180-day forecast**: Long-term predictions

## Usage

### CLI Commands

```bash
# Generate forecasts for all endpoints
mockforge governance forecast

# Forecast for specific service
mockforge governance forecast --service payments

# Forecast for specific endpoint
mockforge governance forecast --endpoint /api/users/{id}

# Forecast with specific window
mockforge governance forecast --window 90  # 90-day forecast
```

### API Usage

```bash
# Get forecasts
GET /api/v1/forecasts?workspace_id=workspace-123&window=90

# Get forecast for specific endpoint
GET /api/v1/forecasts/endpoint?endpoint=/api/users/{id}&method=GET&window=90

# Get service-level forecast
GET /api/v1/forecasts/service?service_id=payments&window=90
```

## Forecast Results

### Example Forecast

```json
{
  "endpoint": "/api/users/{id}",
  "method": "GET",
  "forecast_window_days": 90,
  "predicted_change_probability": 0.75,
  "predicted_break_probability": 0.25,
  "next_expected_change_date": "2025-04-15T00:00:00Z",
  "next_expected_break_date": "2025-05-01T00:00:00Z",
  "volatility_score": 0.6,
  "confidence": 0.8,
  "seasonal_patterns": [
    {
      "pattern_type": "monthly",
      "frequency_days": 30.0,
      "last_occurrence": "2025-01-15T00:00:00Z",
      "confidence": 0.7,
      "description": "Changes typically occur monthly"
    }
  ]
}
```

### Understanding Forecasts

- **predicted_change_probability**: 0.0-1.0, likelihood of any change
- **predicted_break_probability**: 0.0-1.0, likelihood of breaking change
- **volatility_score**: 0.0-1.0, how frequently changes occur (higher = more volatile)
- **confidence**: 0.0-1.0, how reliable the forecast is
- **seasonal_patterns**: Detected patterns in change timing

## Real-World Examples

### Example 1: Monthly Field Additions

**Pattern Detected:**
- This team tends to add fields every 2 weeks
- Changes are non-breaking (new optional fields)
- Pattern confidence: 0.8

**Forecast:**
- 90-day change probability: 0.9
- 90-day break probability: 0.1
- Next expected change: 2 weeks from now

**Action:** Frontend team can prepare for new optional fields.

### Example 2: Quarterly Breaking Changes

**Pattern Detected:**
- This service usually breaks its PATCH contract every quarter
- Changes occur around quarter boundaries
- Pattern confidence: 0.7

**Forecast:**
- 90-day break probability: 0.6
- Next expected break: End of current quarter

**Action:** Platform team can schedule client updates before the break.

### Example 3: Frequent Refactors

**Pattern Detected:**
- This BE team often renames fields during refactors
- Renames occur every 1-2 months
- Pattern confidence: 0.6

**Forecast:**
- 90-day change probability: 0.8
- 90-day break probability: 0.5 (renames are breaking)
- High volatility score: 0.7

**Action:** Consumer teams should implement field mapping strategies.

## Configuration

### Enable Forecasting

```yaml
# mockforge.yaml
contract_drift:
  forecasting:
    enabled: true
    min_incidents_for_forecast: 5  # Need at least 5 incidents
    analysis_windows: [30, 90, 180]  # Days to analyze
    forecast_windows: [30, 90, 180]  # Days to forecast
```

### Forecast Thresholds

```yaml
contract_drift:
  forecasting:
    enabled: true
    thresholds:
      high_volatility: 0.7  # Volatility score threshold
      high_break_probability: 0.5  # Break probability threshold
      low_confidence: 0.5  # Confidence threshold for warnings
```

## Integration with Drift Budgets

Forecasts integrate with drift budgets:

```yaml
contract_drift:
  drift_budget:
    max_breaking_changes: 2
    max_non_breaking_changes: 10
  forecasting:
    enabled: true
    # Forecasts help predict if budget will be exceeded
```

When a forecast predicts budget violations, alerts can be sent proactively.

## Webhooks

Forecast updates can trigger webhooks:

```yaml
webhooks:
  - url: https://slack.com/hooks/...
    events:
      - forecast.prediction_updated
      - forecast.high_volatility_detected
      - forecast.break_probability_high
```

## Best Practices

1. **Collect History**: Ensure sufficient historical data (at least 5 incidents)
2. **Review Regularly**: Check forecasts weekly/monthly
3. **Act on Predictions**: Use forecasts to plan client updates
4. **Track Accuracy**: Monitor forecast accuracy over time
5. **Adjust Thresholds**: Tune thresholds based on your needs

## Troubleshooting

### No Forecasts Available

- **Insufficient History**: Need at least 5 drift incidents
- **No Patterns**: Endpoint may be too stable or too new
- **Low Confidence**: Forecasts may be unreliable

### Inaccurate Forecasts

- **Pattern Changes**: Historical patterns may have changed
- **External Factors**: Events outside normal patterns
- **Low Confidence**: Check confidence scores

## Related Documentation

- [Drift Budgets](../../docs/DRIFT_BUDGETS.md) - Budget management
- [Semantic Drift](semantic-drift.md) - Semantic change detection
- [Contract Threat Modeling](threat-modeling.md) - Security analysis

