#!/usr/bin/env python3
"""
Compare load test results against baseline.

Usage:
    python3 compare_baseline.py <results.json> [baseline.json]
"""

import json
import sys
from pathlib import Path
from datetime import datetime


def load_json(filepath):
    """Load JSON file."""
    with open(filepath, 'r') as f:
        return json.load(f)


def calculate_percentage_change(current, baseline):
    """Calculate percentage change."""
    if baseline == 0:
        return float('inf') if current > 0 else 0.0
    return ((current - baseline) / baseline) * 100


def main():
    if len(sys.argv) < 2:
        print("Usage: compare_baseline.py <results.json> [baseline.json]")
        sys.exit(1)

    results_file = Path(sys.argv[1])
    baseline_file = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    if not results_file.exists():
        print(f"Error: Results file not found: {results_file}")
        sys.exit(1)

    results = load_json(results_file)

    if baseline_file and baseline_file.exists():
        baseline = load_json(baseline_file)
        print("\n=== Performance Comparison vs Baseline ===\n")

        # Compare metrics
        results_metrics = results.get('metrics', {})
        baseline_metrics = baseline.get('metrics', {})

        # HTTP request duration
        if 'http_req_duration' in results_metrics and 'http_req_duration' in baseline_metrics:
            print("HTTP Request Duration:")
            for percentile in ['p50', 'p95', 'p99', 'p99.9']:
                if percentile in results_metrics['http_req_duration'] and percentile in baseline_metrics['http_req_duration']:
                    current = results_metrics['http_req_duration'][percentile]
                    baseline_val = baseline_metrics['http_req_duration'][percentile]
                    change = calculate_percentage_change(current, baseline_val)

                    if abs(change) > 10:  # More than 10% change
                        status = "⚠️" if change > 0 else "✅"
                        print(f"  {status} {percentile}: {current}ms (baseline: {baseline_val}ms, {change:+.1f}%)")
                    else:
                        print(f"  ✅ {percentile}: {current}ms (baseline: {baseline_val}ms, {change:+.1f}%)")

        # HTTP request failure rate
        if 'http_req_failed' in results_metrics and 'http_req_failed' in baseline_metrics:
            current_rate = results_metrics['http_req_failed'].get('rate', 0)
            baseline_rate = baseline_metrics['http_req_failed'].get('rate', 0)
            change = calculate_percentage_change(current_rate, baseline_rate)

            if current_rate > baseline_rate:
                print(f"\n⚠️  Failure Rate: {current_rate:.4f} (baseline: {baseline_rate:.4f}, {change:+.1f}%)")
            else:
                print(f"\n✅ Failure Rate: {current_rate:.4f} (baseline: {baseline_rate:.4f}, {change:+.1f}%)")
    else:
        print("\n=== Saving Current Results as Baseline ===\n")

        # Save as baseline
        baseline_path = results_file.parent / 'baseline.json'
        with open(baseline_path, 'w') as f:
            json.dump(results, f, indent=2)

        print(f"Baseline saved to: {baseline_path}")
        print("\nNext run will compare against this baseline.")

    print()


if __name__ == '__main__':
    main()
