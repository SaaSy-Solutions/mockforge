#!/usr/bin/env python3
"""
Check load test results against performance thresholds.

Usage:
    python3 check_thresholds.py <results.json> <thresholds.json>
"""

import json
import sys
from pathlib import Path


def load_json(filepath):
    """Load JSON file."""
    with open(filepath, 'r') as f:
        return json.load(f)


def check_threshold(metric, value, threshold):
    """Check if a metric value meets the threshold."""
    if threshold.startswith('p('):
        # Percentile threshold: p(95)<500
        percentile = float(threshold.split('(')[1].split(')')[0])
        operator = '<' if '<' in threshold else '>'
        threshold_value = float(threshold.split(operator)[1])

        if operator == '<':
            return value < threshold_value, f"{metric} p{percentile} = {value}ms (threshold: <{threshold_value}ms)"
        else:
            return value > threshold_value, f"{metric} p{percentile} = {value}ms (threshold: >{threshold_value}ms)"
    elif threshold.startswith('rate<'):
        # Rate threshold: rate<0.01
        threshold_value = float(threshold.split('<')[1])
        return value < threshold_value, f"{metric} rate = {value} (threshold: <{threshold_value})"
    else:
        # Simple numeric threshold
        threshold_value = float(threshold)
        return value < threshold_value, f"{metric} = {value} (threshold: <{threshold_value})"


def main():
    if len(sys.argv) != 3:
        print("Usage: check_thresholds.py <results.json> <thresholds.json>")
        sys.exit(1)

    results_file = Path(sys.argv[1])
    thresholds_file = Path(sys.argv[2])

    if not results_file.exists():
        print(f"Error: Results file not found: {results_file}")
        sys.exit(1)

    if not thresholds_file.exists():
        print(f"Error: Thresholds file not found: {thresholds_file}")
        sys.exit(1)

    results = load_json(results_file)
    thresholds = load_json(thresholds_file)

    failed = False
    checks = []

    # Check HTTP request duration
    if 'http_req_duration' in thresholds:
        metrics = results.get('metrics', {})
        http_req_duration = metrics.get('http_req_duration', {})

        for threshold_key, threshold_value in thresholds['http_req_duration'].items():
            if threshold_key in http_req_duration:
                value = http_req_duration[threshold_key]
                passed, message = check_threshold('http_req_duration', value, threshold_value)
                checks.append((passed, message))
                if not passed:
                    failed = True

    # Check HTTP request failure rate
    if 'http_req_failed' in thresholds:
        metrics = results.get('metrics', {})
        http_req_failed = metrics.get('http_req_failed', {})

        if 'rate' in thresholds['http_req_failed']:
            rate = http_req_failed.get('rate', 0)
            threshold = thresholds['http_req_failed']['rate']
            passed, message = check_threshold('http_req_failed', rate, threshold)
            checks.append((passed, message))
            if not passed:
                failed = True

    # Print results
    print("\n=== Performance Threshold Check ===\n")
    for passed, message in checks:
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{status}: {message}")

    if failed:
        print("\n❌ Some performance thresholds were not met!")
        sys.exit(1)
    else:
        print("\n✅ All performance thresholds met!")
        sys.exit(0)


if __name__ == '__main__':
    main()
