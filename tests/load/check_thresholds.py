#!/usr/bin/env python3
"""
Check a k6 end-of-test summary against performance thresholds.

Usage:
    python3 check_thresholds.py <summary.json> <thresholds.json>

<summary.json> is the file written by `k6 run --summary-export=<file>` (or the
`data` object a `handleSummary()` serialises). It is NOT the `--out json=`
stream: that is newline-delimited data points, not a single JSON document.

<thresholds.json> maps metric -> {stat: max}, where `stat` is a k6 summary
stat name (`avg`, `med`, `max`, `p(95)`, `p(99)`, ...) or `rate` for Rate
metrics. Every listed stat must be present in the summary and strictly below
its max. A missing metric or stat is a failure, never a silent skip.
"""

import json
import sys
from pathlib import Path


def metric_stats(metric):
    """Normalise one k6 summary metric to a flat {stat: number} dict.

    --summary-export puts trend stats at the top level and a Rate's ratio
    under `value`; handleSummary data nests them under `values` with the
    ratio under `rate`. Accept both.
    """
    stats = dict(metric.get('values', metric))
    if 'rate' not in stats and 'value' in stats and 'passes' in stats:
        stats['rate'] = stats['value']
    return stats


def main():
    if len(sys.argv) != 3:
        print("Usage: check_thresholds.py <summary.json> <thresholds.json>")
        sys.exit(2)

    summary_file = Path(sys.argv[1])
    thresholds_file = Path(sys.argv[2])
    for f in (summary_file, thresholds_file):
        if not f.exists():
            print(f"Error: file not found: {f}")
            sys.exit(2)

    metrics = json.loads(summary_file.read_text()).get('metrics', {})
    thresholds = json.loads(thresholds_file.read_text())

    failed = False
    print("\n=== Performance Threshold Check ===\n")
    for metric_name, limits in thresholds.items():
        if metric_name not in metrics:
            print(f"FAIL: metric {metric_name} missing from summary")
            failed = True
            continue
        stats = metric_stats(metrics[metric_name])
        for stat, limit in limits.items():
            value = stats.get(stat)
            if value is None:
                print(f"FAIL: {metric_name} {stat} missing from summary "
                      f"(add it to summaryTrendStats)")
                failed = True
                continue
            ok = value < float(limit)
            failed |= not ok
            print(f"{'PASS' if ok else 'FAIL'}: {metric_name} {stat} = {value:.4f} "
                  f"(threshold: <{limit})")

    if failed:
        print("\nSome performance thresholds were not met.")
        sys.exit(1)
    print("\nAll performance thresholds met.")


if __name__ == '__main__':
    main()
