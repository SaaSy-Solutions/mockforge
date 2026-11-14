#!/usr/bin/env python3
"""Fixed-concurrency, fixed-duration HTTP load client with complete accounting."""
import argparse
import concurrent.futures
import json
import time
import urllib.error
import urllib.request


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('url'); parser.add_argument('--concurrency', type=int, default=16)
    parser.add_argument('--duration', type=float, default=15); parser.add_argument('--method', default='GET')
    args = parser.parse_args()
    latencies = []; errors = 0; non_2xx = 0
    deadline = time.monotonic() + args.duration

    def request():
        started = time.perf_counter()
        try:
            request = urllib.request.Request(args.url, method=args.method)
            with urllib.request.urlopen(request, timeout=5) as response:
                response.read()
                return time.perf_counter() - started, 200 <= response.status < 300, False
        except urllib.error.HTTPError as response:
            response.read()
            return time.perf_counter() - started, False, True
        except Exception:
            return time.perf_counter() - started, False, False

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.concurrency) as pool:
        pending = set()
        while time.monotonic() < deadline or pending:
            while time.monotonic() < deadline and len(pending) < args.concurrency:
                pending.add(pool.submit(request))
            if not pending:
                break
            done, pending = concurrent.futures.wait(pending, timeout=.05,
                return_when=concurrent.futures.FIRST_COMPLETED)
            for future in done:
                latency, success, http_error = future.result()
                latencies.append(latency)
                if http_error:
                    non_2xx += 1
                elif not success:
                    errors += 1

    latencies.sort()
    def percentile(q):
        return latencies[min(len(latencies) - 1, int((len(latencies) - 1) * q))] * 1000 if latencies else None
    requests = len(latencies)
    print(json.dumps({'requests': requests, 'throughput_rps': requests / args.duration,
        'p50_ms': percentile(.50), 'p95_ms': percentile(.95), 'p99_ms': percentile(.99),
        'errors': errors, 'non_2xx': non_2xx, 'failed': errors > 0 or non_2xx > 0,
        'concurrency': args.concurrency, 'duration_seconds': args.duration}))


if __name__ == '__main__':
    main()
