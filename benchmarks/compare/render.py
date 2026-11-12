#!/usr/bin/env python3
"""Render one compact, auditable comparison table per fixture."""
import json
import sys

result = json.load(open(sys.argv[1]))
fixtures = sorted({fixture['fixture'] for tool in result['tools'] for fixture in tool['fixtures']})
for name in fixtures:
    print(f'### {name}')
    print('| Tool | Version | Cold start median | Peak RSS (+mode) | Per-operation summary | Fidelity | Status |')
    print('|---|---|---:|---:|---|---:|---|')
    for tool in result['tools']:
        fixture = next((item for item in tool['fixtures'] if item['fixture'] == name), None)
        if fixture is None:
            continue
        metrics = fixture['metrics']
        load = metrics['load']
        operations = load.get('per_operation', [])
        p50s = [item['load']['p50_ms'] for item in operations if item['load'].get('p50_ms') is not None]
        median_p50 = sorted(p50s)[len(p50s) // 2] if p50s else None
        operation_rps = [item['load'].get('throughput_rps') for item in operations
                         if item['load'].get('throughput_rps') is not None]
        rps_median = sorted(operation_rps)[(len(operation_rps) - 1) // 2] if operation_rps else None
        request_count = sum(item['load'].get('requests') or 0 for item in operations)
        percentiles = []
        for key in ('p50_ms', 'p95_ms', 'p99_ms'):
            values = [item['load'][key] for item in operations if item['load'].get(key) is not None]
            percentiles.append(f"{key[:-3]} {sorted(values)[len(values)//2]:.3f} ms" if values else f"{key[:-3]} n/a")
        status_counts = {}
        non_2xx_by_operation = []
        for item in operations:
            operation_bad = []
            counts = item['load'].get('status_counts') or {}
            for code, count in counts.items():
                status_counts[code] = status_counts.get(code, 0) + count
                if not 200 <= int(code) < 300 and count:
                    operation_bad.append(f'{code}×{count}')
            if operation_bad:
                non_2xx_by_operation.append(f"{item['operation']}: {', '.join(operation_bad)}")
        status_text = ', '.join(f'{code}: {status_counts[code]}' for code in sorted(status_counts)) or 'no statuses'
        total_responses = sum(status_counts.values())
        non_2xx = sum(count for code, count in status_counts.items() if not 200 <= int(code) < 300)
        non_2xx_pct = 100 * non_2xx / total_responses if total_responses else 0
        p50_summary = f"operation p50 median {median_p50:.3f} ms" if median_p50 is not None else "operation p50 median n/a"
        rps_summary = f"median {rps_median:.2f} req/s per operation" if rps_median is not None else "median n/a req/s per operation"
        bad_summary = '; non-2xx by operation: ' + '; '.join(non_2xx_by_operation) if non_2xx_by_operation else ''
        summary = f"{'; '.join(percentiles)}; {p50_summary}; {rps_summary}; {request_count} requests total; {status_text}; non-2xx {non_2xx_pct:.2f}%{bad_summary}"
        fidelity = metrics['fidelity']
        fidelity_value = ('n/a' if fidelity.get('status') == 'n/a' else
                          fidelity.get('status') if fidelity.get('status') else
                          f"{fidelity.get('passed', 0)}/{fidelity.get('total', 0)} ({fidelity.get('fraction', 0):.1%})")
        rss = metrics.get('peak_rss_bytes')
        rss_value = f"{rss / (1024 * 1024):.1f} MiB ({metrics.get('peak_rss_mode', 'unknown')})" if rss is not None else 'n/a'
        degraded = total_responses > 0 and non_2xx_pct > 1
        failed = load.get('failed') or any(item['load'].get('exit_code') not in (None, 0) for item in operations)
        if fidelity.get('status') == 'failed':
            failed = True
        cold = metrics.get('cold_start_seconds_median')
        status = 'FAILED' if failed else 'DEGRADED' if degraded else 'OK'
        print(f"| {tool['tool']} | {fixture['version']} | {cold:.3f} s | {rss_value} | {summary} | {fidelity_value} | {status} |")
    print()
