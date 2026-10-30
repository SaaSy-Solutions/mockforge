#!/usr/bin/env python3
"""Release a superseded CI run stuck on its queued, always() completion job.

Dry-run by default. Preserve CI's fail-closed dependency verdict; never cancel
the current PR head, forks, ordinary queued work, or an assigned runner.
"""
import argparse
import json
import subprocess
import time


from superseded_ci import Refused, eligible


def api(path, method="GET"):
    result = subprocess.run(["gh", "api", path, "--method", method], capture_output=True, text=True, timeout=30)
    if result.returncode:
        if method == "POST" and "/cancel" in path and "HTTP 409" in result.stderr:
            return None
        raise RuntimeError("GitHub API request failed; no credential or response body is logged")
    return json.loads(result.stdout) if result.stdout.strip() else None


def snapshot(repo, number, run_id, expected):
    pull = api(f"repos/{repo}/pulls/{number}")
    run = api(f"repos/{repo}/actions/runs/{run_id}")
    pages = subprocess.run(["gh", "api", f"repos/{repo}/actions/runs/{run_id}/jobs?per_page=100", "--paginate", "--slurp"], capture_output=True, text=True, timeout=30)
    if pages.returncode:
        raise RuntimeError("Cannot inspect every workflow job")
    jobs = [job for page in json.loads(pages.stdout) for job in page["jobs"]]
    return eligible(repo, pull, run, jobs, expected)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default="SaaSy-Solutions/mockforge")
    parser.add_argument("--pull-request", type=int, required=True)
    parser.add_argument("--run-id", type=int, required=True)
    parser.add_argument("--expected-head", required=True)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    plan = snapshot(args.repo, args.pull_request, args.run_id, args.expected_head)
    if not args.apply:
        print(json.dumps({**plan, "action": "dry-run"}))
        return
    # Try ordinary cancellation first. Force-cancel is documented specifically
    # for cancellation that cannot finish because an always() job remains.
    api(f"repos/{args.repo}/actions/runs/{args.run_id}/cancel", "POST")
    for _ in range(3):
        run = api(f"repos/{args.repo}/actions/runs/{args.run_id}")
        if run["status"] == "completed":
            print(json.dumps({**plan, "action": "cancelled", "conclusion": run.get("conclusion")}))
            return
        time.sleep(1)
    # Re-fetch PR head and all jobs immediately before the stronger action.
    snapshot(args.repo, args.pull_request, args.run_id, args.expected_head)
    api(f"repos/{args.repo}/actions/runs/{args.run_id}/force-cancel", "POST")
    print(json.dumps({**plan, "action": "force-cancel-requested"}))


if __name__ == "__main__":
    try:
        main()
    except (Refused, RuntimeError, subprocess.TimeoutExpired) as error:
        raise SystemExit(str(error)) from None
